//! 应用共享状态

use crate::p2p::P2PEvent;
use crate::types::ClipItem;
use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct AppState {
    pub connection_status: Mutex<String>,
    pub current_preview: Mutex<String>,
    pub generated_psk: Mutex<Option<String>>,
    pub is_hosting: Mutex<bool>,
    pub is_joining: Mutex<bool>,
    pub remote_addr: Mutex<Option<String>>,
    pub p2p_send: Mutex<Option<mpsc::UnboundedSender<ClipItem>>>,
    pub p2p_event_tx: Mutex<Option<mpsc::UnboundedSender<P2PEvent>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection_status: Mutex::new(String::from("未连接")),
            current_preview: Mutex::new(String::new()),
            generated_psk: Mutex::new(None),
            is_hosting: Mutex::new(false),
            is_joining: Mutex::new(false),
            remote_addr: Mutex::new(None),
            p2p_send: Mutex::new(None),
            p2p_event_tx: Mutex::new(None),
        }
    }

    pub fn set_status(&self, status: impl Into<String>) {
        *self.connection_status.lock().unwrap() = status.into();
    }

    pub fn status(&self) -> String {
        self.connection_status.lock().unwrap().clone()
    }

    pub fn set_preview(&self, preview: impl Into<String>) {
        *self.current_preview.lock().unwrap() = preview.into();
    }

    pub fn preview(&self) -> String {
        self.current_preview.lock().unwrap().clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_constructs_with_defaults() {
        let state = AppState::new();
        assert_eq!(state.status(), "未连接");
        assert!(state.preview().is_empty());
        assert!(state.p2p_send.lock().unwrap().is_none());
    }
}
