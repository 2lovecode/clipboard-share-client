//! ClipboardPort abstraction for text / HTML / image.

use crate::protocol::Payload;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cli_clipboard::{ClipboardContext, ClipboardProvider};

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    Unsupported(String),
    TooLarge(String),
    Platform(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::Unsupported(s)
            | ClipboardError::TooLarge(s)
            | ClipboardError::Platform(s) => write!(f, "{}", s),
        }
    }
}

pub trait ClipboardPort {
    fn read_for_push(&self) -> Result<Payload, ClipboardError>;
    fn write_payload(&mut self, payload: &Payload) -> Result<(), ClipboardError>;
}

/// In-memory clipboard for tests.
#[derive(Debug, Default, Clone)]
pub struct MemoryClipboard {
    pub text: Option<String>,
    pub html: Option<String>,
    pub image: Option<(String, Vec<u8>)>,
    pub reject_files: bool,
    pub image_write_supported: bool,
}

impl MemoryClipboard {
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            image_write_supported: true,
            ..Self::default()
        }
    }
}

impl ClipboardPort for MemoryClipboard {
    fn read_for_push(&self) -> Result<Payload, ClipboardError> {
        if self.reject_files {
            return Err(ClipboardError::Unsupported(
                "file payloads are not supported".into(),
            ));
        }
        if let Some(html) = &self.html {
            return Ok(Payload::Html {
                html: html.clone(),
                plain: self.text.clone(),
            });
        }
        if let Some((mime, bytes)) = &self.image {
            if bytes.len() > MAX_IMAGE_BYTES {
                return Err(ClipboardError::TooLarge(format!(
                    "image exceeds {} bytes",
                    MAX_IMAGE_BYTES
                )));
            }
            return Ok(Payload::Image {
                mime: mime.clone(),
                data_base64: B64.encode(bytes),
            });
        }
        if let Some(text) = &self.text {
            return Ok(Payload::Text {
                text: text.clone(),
            });
        }
        Err(ClipboardError::Platform("clipboard empty".into()))
    }

    fn write_payload(&mut self, payload: &Payload) -> Result<(), ClipboardError> {
        match payload {
            Payload::Text { text } => {
                self.text = Some(text.clone());
                self.html = None;
                Ok(())
            }
            Payload::Html { html, plain } => {
                self.html = Some(html.clone());
                if let Some(p) = plain {
                    self.text = Some(p.clone());
                }
                Ok(())
            }
            Payload::Image { mime, data_base64 } => {
                if !self.image_write_supported {
                    return Err(ClipboardError::Platform(
                        "image clipboard write is not supported on this platform".into(),
                    ));
                }
                let bytes = B64
                    .decode(data_base64)
                    .map_err(|e| ClipboardError::Platform(e.to_string()))?;
                if bytes.len() > MAX_IMAGE_BYTES {
                    return Err(ClipboardError::TooLarge(format!(
                        "image exceeds {} bytes",
                        MAX_IMAGE_BYTES
                    )));
                }
                self.image = Some((mime.clone(), bytes));
                Ok(())
            }
        }
    }
}

/// System clipboard — text via cli-clipboard; HTML/image best-effort.
pub struct SystemClipboard {
    ctx: ClipboardContext,
}

impl SystemClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        ClipboardContext::new()
            .map(|ctx| Self { ctx })
            .map_err(|e| ClipboardError::Platform(e.to_string()))
    }
}

impl ClipboardPort for SystemClipboard {
    fn read_for_push(&self) -> Result<Payload, ClipboardError> {
        // cli-clipboard only exposes text; prefer that as plain text push.
        // HTML/image capture is platform-specific; fall back to text.
        let mut ctx = ClipboardContext::new()
            .map_err(|e| ClipboardError::Platform(e.to_string()))?;
        let text = ctx
            .get_contents()
            .map_err(|e| ClipboardError::Platform(e.to_string()))?;
        if text.is_empty() {
            return Err(ClipboardError::Platform("clipboard empty".into()));
        }
        // Heuristic: if content looks like HTML, treat as rich text.
        let trimmed = text.trim_start();
        if trimmed.starts_with('<') && trimmed.contains('>') {
            Ok(Payload::Html {
                html: text.clone(),
                plain: Some(strip_tags_rough(&text)),
            })
        } else {
            Ok(Payload::Text { text })
        }
    }

    fn write_payload(&mut self, payload: &Payload) -> Result<(), ClipboardError> {
        match payload {
            Payload::Text { text } => self
                .ctx
                .set_contents(text.clone())
                .map_err(|e| ClipboardError::Platform(e.to_string())),
            Payload::Html { html, plain } => {
                // Best-effort: write plain if present, else html source as text.
                let content = plain.clone().unwrap_or_else(|| html.clone());
                self.ctx
                    .set_contents(content)
                    .map_err(|e| ClipboardError::Platform(e.to_string()))
            }
            Payload::Image { .. } => Err(ClipboardError::Platform(
                "image clipboard write is not supported on this platform".into(),
            )),
        }
    }
}

fn strip_tags_rough(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Reject explicit file attempts at the app boundary.
pub fn reject_file_payload() -> ClipboardError {
    ClipboardError::Unsupported("file payloads are not supported".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        let clip = MemoryClipboard::with_text("hi");
        let payload = clip.read_for_push().unwrap();
        let mut other = MemoryClipboard::default();
        other.image_write_supported = true;
        other.write_payload(&payload).unwrap();
        assert_eq!(other.text.as_deref(), Some("hi"));
    }

    #[test]
    fn prefers_html_when_available() {
        let clip = MemoryClipboard {
            text: Some("plain".into()),
            html: Some("<b>x</b>".into()),
            image_write_supported: true,
            ..Default::default()
        };
        match clip.read_for_push().unwrap() {
            Payload::Html { html, plain } => {
                assert_eq!(html, "<b>x</b>");
                assert_eq!(plain.as_deref(), Some("plain"));
            }
            other => panic!("expected html, got {:?}", other),
        }
    }

    #[test]
    fn falls_back_to_plain_text() {
        let clip = MemoryClipboard::with_text("only-text");
        match clip.read_for_push().unwrap() {
            Payload::Text { text } => assert_eq!(text, "only-text"),
            other => panic!("expected text, got {:?}", other),
        }
    }

    #[test]
    fn rejects_files() {
        let clip = MemoryClipboard {
            reject_files: true,
            ..Default::default()
        };
        let err = clip.read_for_push().unwrap_err();
        assert!(matches!(err, ClipboardError::Unsupported(_)));
        let err = reject_file_payload();
        assert!(err.to_string().contains("file"));
    }

    #[test]
    fn image_write_success() {
        let mut clip = MemoryClipboard {
            image_write_supported: true,
            ..Default::default()
        };
        let payload = Payload::Image {
            mime: "image/png".into(),
            data_base64: B64.encode([1, 2, 3, 4]),
        };
        clip.write_payload(&payload).unwrap();
        assert!(clip.image.is_some());
    }

    #[test]
    fn image_write_unsupported_is_clear() {
        let mut clip = MemoryClipboard {
            image_write_supported: false,
            ..Default::default()
        };
        let payload = Payload::Image {
            mime: "image/png".into(),
            data_base64: B64.encode([1, 2, 3]),
        };
        let err = clip.write_payload(&payload).unwrap_err();
        assert!(err.to_string().contains("not supported"));
    }

    #[test]
    fn image_too_large_rejected_on_read() {
        let big = vec![0u8; MAX_IMAGE_BYTES + 1];
        let clip = MemoryClipboard {
            image: Some(("image/png".into(), big)),
            image_write_supported: true,
            ..Default::default()
        };
        let err = clip.read_for_push().unwrap_err();
        assert!(matches!(err, ClipboardError::TooLarge(_)));
    }
}
