//! 全局热键监听模块（暂时禁用，等 core-graphics API 问题修复）

use crate::BackendEvent;
use tokio::sync::mpsc;

/// 初始化全局热键监听（暂时禁用）
pub fn spawn_global_listener(_tx: mpsc::UnboundedSender<BackendEvent>) {
    println!("[hotkey] Global hotkey listener disabled - needs fix");
    // TODO: 重新实现全局热键监听
    // 问题：core-graphics v0.24 的 CGEventTap API 变化较大
}
