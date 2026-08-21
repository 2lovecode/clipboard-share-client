//! In-memory clipboard share history.

use crate::protocol::Payload;
use uuid::Uuid;

pub const DEFAULT_CAPACITY: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryItem {
    pub id: String,
    pub source: Source,
    pub payload: Payload,
    pub summary: String,
}

#[derive(Debug, Default)]
pub struct HistoryStore {
    items: Vec<HistoryItem>,
    capacity: usize,
    /// Outbound messages queued by explicit push (not by click).
    outbound: Vec<HistoryItem>,
}

impl HistoryStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::new(),
            capacity: capacity.max(1),
            outbound: Vec::new(),
        }
    }

    pub fn items(&self) -> &[HistoryItem] {
        &self.items
    }

    pub fn take_outbound(&mut self) -> Vec<HistoryItem> {
        std::mem::take(&mut self.outbound)
    }

    pub fn peek_outbound(&self) -> &[HistoryItem] {
        &self.outbound
    }

    /// Explicit local push: enqueue locally and mark for peer send.
    pub fn push_local(&mut self, payload: Payload) -> HistoryItem {
        let item = HistoryItem {
            id: Uuid::new_v4().to_string(),
            source: Source::Local,
            summary: summarize(&payload),
            payload,
        };
        self.outbound.push(item.clone());
        self.push_item(item.clone());
        item
    }

    /// System clipboard changed — MUST NOT enqueue.
    pub fn on_clipboard_changed(&mut self, _payload: Payload) {
        // intentional no-op per clipboard-history spec
    }

    pub fn receive_remote(&mut self, id: String, payload: Payload) -> HistoryItem {
        let item = HistoryItem {
            id,
            source: Source::Remote,
            summary: summarize(&payload),
            payload,
        };
        self.push_item(item.clone());
        item
    }

    /// Click to write local clipboard — MUST NOT create outbound.
    pub fn select_for_local_clipboard(&self, index: usize) -> Option<&HistoryItem> {
        self.items.get(index)
    }

    fn push_item(&mut self, item: HistoryItem) {
        self.items.push(item);
        while self.items.len() > self.capacity {
            self.items.remove(0);
        }
    }
}

fn summarize(payload: &Payload) -> String {
    match payload {
        Payload::Text { text } => {
            let t = text.trim();
            if t.chars().count() > 40 {
                format!("{}…", t.chars().take(40).collect::<String>())
            } else {
                t.to_string()
            }
        }
        Payload::Html { html, plain } => plain
            .clone()
            .unwrap_or_else(|| format!("HTML ({} bytes)", html.len())),
        Payload::Image { mime, .. } => format!("Image ({})", mime),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Payload {
        Payload::Text {
            text: s.to_string(),
        }
    }

    #[test]
    fn explicit_push_enqueues_local_and_outbound() {
        let mut store = HistoryStore::new(10);
        store.push_local(text("hello"));
        assert_eq!(store.items().len(), 1);
        assert_eq!(store.items()[0].source, Source::Local);
        assert_eq!(store.peek_outbound().len(), 1);
    }

    #[test]
    fn clipboard_change_does_not_enqueue() {
        let mut store = HistoryStore::new(10);
        store.on_clipboard_changed(text("ghost"));
        assert!(store.items().is_empty());
        assert!(store.peek_outbound().is_empty());
    }

    #[test]
    fn select_does_not_create_outbound() {
        let mut store = HistoryStore::new(10);
        store.push_local(text("a"));
        let _ = store.take_outbound();
        assert!(store.select_for_local_clipboard(0).is_some());
        assert!(store.peek_outbound().is_empty());
    }

    #[test]
    fn remote_source_distinguishable() {
        let mut store = HistoryStore::new(10);
        store.push_local(text("local"));
        store.receive_remote("r1".into(), text("remote"));
        assert_eq!(store.items()[0].source, Source::Local);
        assert_eq!(store.items()[1].source, Source::Remote);
    }

    #[test]
    fn drops_oldest_when_over_capacity() {
        let mut store = HistoryStore::new(2);
        store.push_local(text("1"));
        store.push_local(text("2"));
        store.push_local(text("3"));
        assert_eq!(store.items().len(), 2);
        assert!(matches!(
            &store.items()[0].payload,
            Payload::Text { text } if text == "2"
        ));
    }
}
