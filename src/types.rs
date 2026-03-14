//! 剪切板条目类型：文字 / 图片，用于 P2P 传输与本地历史

use serde::{Deserialize, Serialize};

/// 一条剪切板内容（文字或图片）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipItem {
    Text(String),
    Image {
        width: u32,
        height: u32,
        #[serde(with = "base64_bytes")]
        bytes: Vec<u8>,
    },
}

mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let b64 = BASE64.encode(v);
        b64.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        BASE64.decode(s.as_bytes()).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl ClipItem {
    pub fn text_preview(&self, max_len: usize) -> String {
        match self {
            ClipItem::Text(s) => {
                let s = s.trim();
                if s.len() <= max_len {
                    s.to_string()
                } else {
                    format!("{}...", &s[..max_len])
                }
            }
            ClipItem::Image { width, height, .. } => format!("[图片 {}×{}]", width, height),
        }
    }
}
