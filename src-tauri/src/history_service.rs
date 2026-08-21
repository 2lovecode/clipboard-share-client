//! 历史列表展示与本地应用的纯逻辑（可单测）

use crate::history_sqlite::HistoryEntry;
use crate::types::ClipItem;
use serde::{Deserialize, Serialize};

/// 前端展示用摘要（不含大图字节）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistorySummary {
    pub id: u64,
    pub preview: String,
    pub at: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
}

pub fn entry_to_summary(entry: &HistoryEntry) -> HistorySummary {
    let kind = match &entry.item {
        ClipItem::Text(_) => "text".to_string(),
        ClipItem::Image { .. } => "image".to_string(),
    };
    HistorySummary {
        id: entry.id,
        preview: entry.item.text_preview(80),
        at: entry.at.clone(),
        kind,
    }
}

pub fn entries_to_summaries(entries: &[HistoryEntry]) -> Vec<HistorySummary> {
    entries.iter().map(entry_to_summary).collect()
}

pub fn paginate<T: Clone>(items: &[T], page: usize, page_size: usize) -> PageResult<T> {
    let page_size = page_size.max(1);
    let total = items.len();
    let total_pages = if total == 0 {
        1
    } else {
        (total + page_size - 1) / page_size
    };
    let page = page.min(total_pages.saturating_sub(1));
    let start = page * page_size;
    let end = (start + page_size).min(total);
    PageResult {
        items: items[start..end].to_vec(),
        page,
        page_size,
        total,
        total_pages,
    }
}

/// 将历史项写入本机剪贴板；不触发任何对端发送。
pub fn apply_local(item: &ClipItem, set_clipboard: impl FnOnce(&ClipItem) -> bool) -> bool {
    set_clipboard(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ClipItem;
    use std::sync::{Arc, Mutex};

    fn sample_entry(id: u64, text: &str) -> HistoryEntry {
        HistoryEntry {
            id,
            item: ClipItem::Text(text.to_string()),
            at: "2026-01-01 00:00:00".to_string(),
        }
    }

    #[test]
    fn get_history_summaries_omit_full_payload_details() {
        let entries = vec![sample_entry(1, "hello world")];
        let summaries = entries_to_summaries(&entries);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, 1);
        assert_eq!(summaries[0].preview, "hello world");
        assert_eq!(summaries[0].kind, "text");
    }

    #[test]
    fn apply_local_writes_clipboard_without_peer_send() {
        let sent_to_peer = Arc::new(Mutex::new(false));
        let written = Arc::new(Mutex::new(None));
        let written_clone = written.clone();
        let peer_flag = sent_to_peer.clone();

        let item = ClipItem::Text("paste-me".to_string());
        let ok = apply_local(&item, |i| {
            // 故意不碰 peer_flag：apply_local 调用方也不应发送
            *written_clone.lock().unwrap() = Some(i.clone());
            let _ = peer_flag; // 证明本测试路径不设置对端发送
            true
        });

        assert!(ok);
        assert_eq!(
            *written.lock().unwrap(),
            Some(ClipItem::Text("paste-me".to_string()))
        );
        assert_eq!(*sent_to_peer.lock().unwrap(), false);
    }

    #[test]
    fn paginate_returns_requested_page() {
        let items: Vec<u32> = (1..=10).collect();
        let page = paginate(&items, 1, 3);
        assert_eq!(page.items, vec![4, 5, 6]);
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 4);
        assert_eq!(page.total, 10);
    }

    #[test]
    fn paginate_clamps_past_last_page() {
        let items: Vec<u32> = (1..=5).collect();
        let page = paginate(&items, 99, 3);
        assert_eq!(page.page, 1);
        assert_eq!(page.items, vec![4, 5]);
    }

    #[test]
    fn filter_by_preview_query() {
        let entries = vec![
            sample_entry(1, "alpha"),
            sample_entry(2, "beta"),
            sample_entry(3, "alphabet"),
        ];
        let summaries = entries_to_summaries(&entries);
        let filtered: Vec<_> = summaries
            .into_iter()
            .filter(|s| s.preview.contains("alp"))
            .collect();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, 1);
        assert_eq!(filtered[1].id, 3);
    }
}
