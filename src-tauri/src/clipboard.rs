//! 系统剪切板：读写文字与图片，监听变化

use crate::types::ClipItem;
use arboard::Clipboard;
use tokio::sync::mpsc;

/// 从系统剪切板读取当前内容（文字优先，否则尝试图片）
pub fn get_current() -> Option<ClipItem> {
    let mut clipboard = Clipboard::new().ok()?;
    if let Ok(s) = clipboard.get_text() {
        if !s.is_empty() {
            return Some(ClipItem::Text(s));
        }
    }
    if let Ok(img) = clipboard.get_image() {
        return Some(ClipItem::Image {
            width: img.width as u32,
            height: img.height as u32,
            bytes: img.bytes.to_vec(),
        });
    }
    None
}

/// 写入系统剪切板（文字或图片）
pub fn set(item: &ClipItem) -> bool {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    match item {
        ClipItem::Text(s) => clipboard.set_text(s.as_str()).is_ok(),
        ClipItem::Image { width, height, bytes } => clipboard
            .set_image(arboard::ImageData {
                width: *width as usize,
                height: *height as usize,
                bytes: bytes.as_slice().into(),
            })
            .is_ok(),
    }
}

/// 在后台轮询剪切板变化，通过 tx 发送 ClipItem（仅在有新内容时）
pub fn spawn_watcher(tx: mpsc::UnboundedSender<ClipItem>) {
    std::thread::spawn(move || {
        let mut last: Option<Vec<u8>> = None;
        let _clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        loop {
            let current = get_current();
            let current_hash = current.as_ref().map(|c| hash_item(c));
            if current_hash != last {
                last = current_hash;
                if let Some(item) = current {
                    let _ = tx.send(item);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}

fn hash_item(item: &ClipItem) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    match item {
        ClipItem::Text(s) => s.hash(&mut hasher),
        ClipItem::Image { width, height, bytes } => {
            width.hash(&mut hasher);
            height.hash(&mut hasher);
            bytes.len().hash(&mut hasher);
            if bytes.len() <= 4096 {
                bytes.hash(&mut hasher);
            } else {
                bytes[..4096].hash(&mut hasher);
            }
        }
    }
    hasher.finish().to_le_bytes().to_vec()
}
