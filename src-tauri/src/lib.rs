mod clipboard;
mod commands;
mod history_service;
mod history_sqlite;
mod notification;
mod p2p;
mod state;
mod types;

use commands::{
    apply_history_item, delete_history_item, disconnect, get_connection_status,
    get_current_preview, get_history, handle_clipboard_changed, handle_p2p_event, search_history,
    start_host, start_join,
};
use p2p::P2PEvent;
use state::AppState;
use std::sync::OnceLock;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tokio::sync::mpsc;
use types::ClipItem;

static RUNTIME_HANDLE: OnceLock<tokio::runtime::Handle> = OnceLock::new();

pub fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    for _ in 0..50 {
        if let Some(handle) = RUNTIME_HANDLE.get() {
            let _guard = handle.enter();
            return f();
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    f()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_history,
            search_history,
            apply_history_item,
            delete_history_item,
            get_connection_status,
            get_current_preview,
            start_host,
            start_join,
            disconnect,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();

            let (p2p_event_tx, mut p2p_event_rx) = mpsc::unbounded_channel::<P2PEvent>();
            let (clip_tx, mut clip_rx) = mpsc::unbounded_channel::<ClipItem>();

            *state.p2p_event_tx.lock().unwrap() = Some(p2p_event_tx);

            let bridge_handle = handle.clone();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[runtime] failed to create tokio runtime: {}", e);
                        return;
                    }
                };
                let app_for_handle = bridge_handle.clone();
                rt.block_on(async move {
                    let _ = RUNTIME_HANDLE.set(tokio::runtime::Handle::current());
                    let state = app_for_handle.state::<AppState>();
                    loop {
                        tokio::select! {
                            Some(ev) = p2p_event_rx.recv() => {
                                handle_p2p_event(&app_for_handle, &state, ev);
                            }
                            Some(item) = clip_rx.recv() => {
                                handle_clipboard_changed(&app_for_handle, &state, item);
                            }
                            else => break,
                        }
                    }
                });
            });

            clipboard::spawn_watcher(clip_tx);
            register_hotkeys(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn register_hotkeys(app: &tauri::App) {
    let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
    let app_handle = app.handle().clone();
    if let Err(e) = app.global_shortcut().on_shortcut(toggle, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Some(window) = app_handle.get_webview_window("main") {
                match window.is_visible() {
                    Ok(true) => {
                        let _ = window.hide();
                    }
                    Ok(false) => {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    Err(err) => eprintln!("[hotkey] window visibility error: {}", err),
                }
            }
        }
    }) {
        eprintln!("[hotkey] failed to register toggle shortcut: {}", e);
    }

    let digit_codes = [
        Code::Digit1,
        Code::Digit2,
        Code::Digit3,
        Code::Digit4,
        Code::Digit5,
        Code::Digit6,
        Code::Digit7,
        Code::Digit8,
        Code::Digit9,
    ];

    for (idx, code) in digit_codes.into_iter().enumerate() {
        let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), code);
        let app_handle = app.handle().clone();
        if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_app, _s, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let entries = history_sqlite::load();
            if let Some(entry) = entries.get(idx) {
                let _ = history_service::apply_local(&entry.item, |i| clipboard::set(i));
                let _ = app_handle.emit("clipboard-preview-updated", entry.item.text_preview(80));
            }
        }) {
            eprintln!("[hotkey] failed to register quick-copy {}: {}", idx + 1, e);
        }
    }
}
