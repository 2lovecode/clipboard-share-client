//! 本地剪切板历史：持久化、搜索、列表

use crate::types::ClipItem;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;

const MAX_ENTRIES: usize = 200;
const HISTORY_FILE: &str = "clipboard_history.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    #[serde(flatten)]
    pub item: ClipItem,
    pub at: String, // ISO8601
}

fn history_path() -> std::path::PathBuf {
    let dir = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join(".clipboard-share").join(HISTORY_FILE)
}

/// 确保数据目录存在
fn ensure_dir() -> std::io::Result<std::path::PathBuf> {
    let p = history_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(p)
}

/// 从磁盘加载历史（按时间倒序，最新在前）
pub fn load() -> Vec<HistoryEntry> {
    let path = history_path();
    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let raw: Vec<HistoryEntry> = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    raw.into_iter().take(MAX_ENTRIES).collect()
}

/// 追加一条并写回磁盘
pub fn add(item: ClipItem) {
    let path = match ensure_dir() {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut entries = load();
    let id = entries
        .first()
        .map(|e| e.id + 1)
        .unwrap_or(1);
    let at = Utc::now().to_rfc3339();
    entries.insert(
        0,
        HistoryEntry {
            id,
            item,
            at,
        },
    );
    let entries: Vec<_> = entries.into_iter().take(MAX_ENTRIES).collect();
    let _ = fs::write(
        &path,
        serde_json::to_string_pretty(&entries).unwrap_or_default(),
    );
}

/// 按关键词搜索（仅匹配文字内容；图片按 "[图片]" 占位）
pub fn search(query: &str) -> Vec<HistoryEntry> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return load();
    }
    load()
        .into_iter()
        .filter(|e| e.item.text_preview(2000).to_lowercase().contains(&q))
        .collect()
}

/// 最近 N 条
pub fn list_recent(n: usize) -> Vec<HistoryEntry> {
    load().into_iter().take(n).collect()
}
