//! 本地剪切板历史：SQLite 持久化、搜索、列表

use crate::types::ClipItem;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const MAX_ENTRIES: usize = 200;
const DB_NAME: &str = "clipboard_history.db";

/// 历史条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    #[serde(flatten)]
    pub item: ClipItem,
    pub at: String,
}

/// 获取数据库路径
fn get_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("无法找到主目录")
        .join(".clipboard_share")
        .join(DB_NAME)
}

/// 获取数据库连接
fn get_conn() -> Connection {
    let db_path = get_db_path();

    // 确保目录存在
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let conn = Connection::open(&db_path).expect("无法打开数据库");

    // 创建表
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_hash TEXT NOT NULL,
            content_data TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )",
        [],
    );

    conn
}

/// 计算内容哈希
fn hash_content(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 添加历史记录
pub fn add(item: ClipItem) {
    let content_data = serde_json::to_string(&item).unwrap_or_default();
    let content_hash = hash_content(&content_data);
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let conn = get_conn();

    // 检查是否已存在相同内容
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM history WHERE content_hash = ?)",
            params![content_hash],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        let _ = conn.execute(
            "INSERT INTO history (content_hash, content_data, timestamp)
             VALUES (?, ?, ?)",
            params![content_hash, content_data, timestamp],
        );
    } else {
        // 更新时间戳
        let _ = conn.execute(
            "UPDATE history SET timestamp = ? WHERE content_hash = ?",
            params![timestamp, content_hash],
        );
    }

    // 清理旧记录
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0)).unwrap_or(0);
    if count > MAX_ENTRIES as i64 {
        let delete_count = count - MAX_ENTRIES as i64;
        let _ = conn.execute(
            "DELETE FROM history WHERE id IN (
                SELECT id FROM history ORDER BY timestamp ASC LIMIT ?
            )",
            params![delete_count],
        );
    }
}

/// 加载所有历史记录
pub fn load() -> Vec<HistoryEntry> {
    let conn = get_conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, content_data, timestamp FROM history ORDER BY timestamp DESC LIMIT ?",
        )
        .unwrap();

    let result = stmt.query_map(params![MAX_ENTRIES as i64], |row| {
        let id: u64 = row.get(0)?;
        let content_data: String = row.get(1)?;
        let at: String = row.get(2)?;

        let item: ClipItem = serde_json::from_str(&content_data).unwrap_or_else(|_| {
            ClipItem::Text(format!("[无法解析]"))
        });

        Ok(HistoryEntry { id, item, at })
    });

    match result {
        Ok(mapped) => {
            let collected: Result<Vec<HistoryEntry>, _> = mapped.collect();
            collected.unwrap_or(Vec::new())
        }
        Err(_) => Vec::new(),
    }
}

/// 搜索历史记录
pub fn search(query: &str) -> Vec<HistoryEntry> {
    let conn = get_conn();

    if query.is_empty() {
        return load();
    }

    let pattern = format!("%{}%", query);

    let mut stmt = conn
        .prepare(
            "SELECT id, content_data, timestamp FROM history
             WHERE content_data LIKE ? ORDER BY timestamp DESC LIMIT ?",
        )
        .unwrap();

    let result = stmt.query_map(params![pattern, MAX_ENTRIES as i64], |row| {
        let id: u64 = row.get(0)?;
        let content_data: String = row.get(1)?;
        let at: String = row.get(2)?;

        let item: ClipItem = serde_json::from_str(&content_data).unwrap_or_else(|_| {
            ClipItem::Text(format!("[无法解析]"))
        });

        Ok(HistoryEntry { id, item, at })
    });

    match result {
        Ok(mapped) => {
            let collected: Result<Vec<HistoryEntry>, _> = mapped.collect();
            collected.unwrap_or(Vec::new())
        }
        Err(_) => Vec::new(),
    }
}

/// 删除历史记录
pub fn delete(id: u64) -> bool {
    let conn = get_conn();
    match conn.execute("DELETE FROM history WHERE id = ?", params![id]) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// 列出最近 N 条记录
#[allow(dead_code)]
pub fn list_recent(n: usize) -> Vec<HistoryEntry> {
    let conn = get_conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, content_data, timestamp FROM history ORDER BY timestamp DESC LIMIT ?",
        )
        .unwrap();

    let result = stmt.query_map(params![n as i64], |row| {
        let id: u64 = row.get(0)?;
        let content_data: String = row.get(1)?;
        let at: String = row.get(2)?;

        let item: ClipItem = serde_json::from_str(&content_data).unwrap_or_else(|_| {
            ClipItem::Text(format!("[无法解析]"))
        });

        Ok(HistoryEntry { id, item, at })
    });

    match result {
        Ok(mapped) => {
            let collected: Result<Vec<HistoryEntry>, _> = mapped.collect();
            collected.unwrap_or(Vec::new())
        }
        Err(_) => Vec::new(),
    }
}
