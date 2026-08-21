//! Tauri commands：历史、连接、预览

use crate::clipboard as cb;
use crate::history_service::{entries_to_summaries, paginate, HistorySummary, PageResult};
use crate::history_sqlite;
use crate::p2p;
use crate::state::AppState;
use crate::types::ClipItem;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub status: String,
    pub generated_psk: Option<String>,
    pub is_hosting: bool,
    pub is_joining: bool,
    pub preview: String,
}

#[tauri::command]
pub fn get_history() -> Result<Vec<HistorySummary>, String> {
    let entries = history_sqlite::load();
    Ok(entries_to_summaries(&entries))
}

#[tauri::command]
pub fn search_history(query: String, page: usize, page_size: usize) -> Result<PageResult<HistorySummary>, String> {
    let entries = history_sqlite::search(&query);
    let summaries = entries_to_summaries(&entries);
    Ok(paginate(&summaries, page, page_size))
}

#[tauri::command]
pub fn apply_history_item(id: u64) -> Result<bool, String> {
    let entries = history_sqlite::load();
    let entry = entries
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "历史项不存在".to_string())?;
    // 规格：仅写本机，不因选择而回传对端
    Ok(crate::history_service::apply_local(&entry.item, |i| cb::set(i)))
}

#[tauri::command]
pub fn delete_history_item(id: u64) -> Result<bool, String> {
    Ok(history_sqlite::delete(id))
}

#[tauri::command]
pub fn get_connection_status(state: State<'_, AppState>) -> Result<ConnectionInfo, String> {
    Ok(ConnectionInfo {
        status: state.status(),
        generated_psk: state.generated_psk.lock().unwrap().clone(),
        is_hosting: *state.is_hosting.lock().unwrap(),
        is_joining: *state.is_joining.lock().unwrap(),
        preview: state.preview(),
    })
}

#[tauri::command]
pub fn get_current_preview(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.preview())
}

#[tauri::command]
pub fn start_host(app: AppHandle, state: State<'_, AppState>, port: u16) -> Result<(), String> {
    if !(1..=65535).contains(&port) {
        return Err("端口号必须在 1-65535 范围内".to_string());
    }
    let event_tx = state
        .p2p_event_tx
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "P2P 事件通道未初始化".to_string())?;
    let send = crate::with_runtime(|| p2p::run_host(port, event_tx));
    *state.p2p_send.lock().unwrap() = Some(send);
    *state.is_hosting.lock().unwrap() = true;
    *state.is_joining.lock().unwrap() = false;
    *state.generated_psk.lock().unwrap() = None;
    state.set_status(format!("监听 0.0.0.0:{} …", port));
    let _ = app.emit("connection-changed", state.status());
    Ok(())
}

#[tauri::command]
pub fn start_join(
    app: AppHandle,
    state: State<'_, AppState>,
    addr: String,
    psk: String,
) -> Result<(), String> {
    if psk.is_empty() {
        return Err("请输入 PSK".to_string());
    }
    let event_tx = state
        .p2p_event_tx
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "P2P 事件通道未初始化".to_string())?;
    let send = crate::with_runtime(|| p2p::run_join(addr.clone(), psk, event_tx));
    *state.p2p_send.lock().unwrap() = Some(send);
    *state.is_joining.lock().unwrap() = true;
    *state.is_hosting.lock().unwrap() = false;
    *state.remote_addr.lock().unwrap() = Some(addr);
    state.set_status("正在连接…");
    let _ = app.emit("connection-changed", state.status());
    Ok(())
}

#[tauri::command]
pub fn disconnect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    *state.p2p_send.lock().unwrap() = None;
    *state.is_hosting.lock().unwrap() = false;
    *state.is_joining.lock().unwrap() = false;
    *state.generated_psk.lock().unwrap() = None;
    *state.remote_addr.lock().unwrap() = None;
    state.set_status("已断开");
    let _ = app.emit("connection-changed", state.status());
    Ok(())
}

pub fn handle_p2p_event(app: &AppHandle, state: &AppState, event: p2p::P2PEvent) {
    match event {
        p2p::P2PEvent::Connected => {
            *state.is_joining.lock().unwrap() = false;
            *state.is_hosting.lock().unwrap() = false;
            let addr = state.remote_addr.lock().unwrap().clone().unwrap_or_default();
            let status = if addr.is_empty() {
                "已连接".to_string()
            } else {
                format!("已连接 to {}", addr)
            };
            state.set_status(status.clone());
            crate::notification::show_connected();
            let _ = app.emit("connection-changed", status);
        }
        p2p::P2PEvent::Disconnected(reason) => {
            *state.is_hosting.lock().unwrap() = false;
            *state.is_joining.lock().unwrap() = false;
            *state.p2p_send.lock().unwrap() = None;
            let status = format!("已断开: {}", reason);
            state.set_status(status.clone());
            crate::notification::show_disconnected(&reason);
            let _ = app.emit("connection-changed", status);
        }
        p2p::P2PEvent::Received(item) => {
            history_sqlite::add(item.clone());
            let _ = cb::set(&item);
            let preview = item.text_preview(80);
            state.set_preview(preview.clone());
            let _ = app.emit("clipboard-preview-updated", preview);
            let _ = app.emit("history-updated", ());
            crate::notification::show_sync_received();
        }
        p2p::P2PEvent::PskGenerated(psk) => {
            *state.generated_psk.lock().unwrap() = Some(psk.clone());
            let status = format!("PSK: {} (复制此密钥给 Join 方)", psk);
            state.set_status(status.clone());
            let _ = app.emit("psk-generated", psk);
            let _ = app.emit("connection-changed", status);
        }
    }
}

pub fn handle_clipboard_changed(app: &AppHandle, state: &AppState, item: ClipItem) {
    if item.is_image_too_large() {
        state.set_status("图片过大（最大 4MB），不同步");
        let _ = app.emit("connection-changed", state.status());
        return;
    }
    history_sqlite::add(item.clone());
    if let Some(ref send) = *state.p2p_send.lock().unwrap() {
        let _ = send.send(item.clone());
        crate::notification::show_sync_sent();
    }
    let preview = item.text_preview(80);
    state.set_preview(preview.clone());
    let _ = app.emit("clipboard-preview-updated", preview);
    let _ = app.emit("history-updated", ());
}
