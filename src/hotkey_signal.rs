//! Global hotkey signal for explicit clipboard push (Alt+C).

use std::sync::atomic::{AtomicBool, Ordering};

static PUSH_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_push() {
    PUSH_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn take_push_requested() -> bool {
    PUSH_REQUESTED.swap(false, Ordering::SeqCst)
}
