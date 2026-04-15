//! 系统通知模块

use notify_rust::Notification;

#[allow(dead_code)]

/// 显示剪贴板同步通知
pub fn show_sync_sent() {
    let _ = Notification::new()
        .summary("剪贴板已同步")
        .body("已同步到对端")
        .show();
}

#[allow(dead_code)]
/// 显示收到同步内容通知
pub fn show_sync_received() {
    let _ = Notification::new()
        .summary("剪贴板同步")
        .body("已收到同步内容")
        .show();
}

#[allow(dead_code)]
/// 显示连接状态通知
pub fn show_connected() {
    let _ = Notification::new()
        .summary("连接状态")
        .body("已连接到对端")
        .show();
}

#[allow(dead_code)]
/// 显示断开连接通知
pub fn show_disconnected(reason: &str) {
    let _ = Notification::new()
        .summary("连接已断开")
        .body(reason)
        .show();
}
