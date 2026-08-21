//! 剪切板共享客户端：P2P 加密同步 + 系统剪切板接管 + 本地历史与快捷键

mod clipboard;
mod history_sqlite;
mod hotkey;
mod notification;
mod p2p;
mod types;

use crate::clipboard as cb;
use crate::p2p::P2PEvent;
use crate::types::ClipItem;
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 后台事件：P2P 或剪切板变化
#[derive(Debug, Clone)]
enum BackendEvent {
    P2P(P2PEvent),
    ClipboardChanged(ClipItem),
    ToggleWindow,
    QuickCopy(usize),
}

struct ClipboardShare {
    to_ui_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<BackendEvent>>>>,
    p2p_event_tx: mpsc::UnboundedSender<P2PEvent>,
    p2p_send: Option<mpsc::UnboundedSender<ClipItem>>,
    connection_status: String,
    host_port_input: String,
    join_addr_input: String,
    psk_input: String,
    generated_psk: Option<String>,
    is_hosting: bool,
    is_joining: bool,
    remote_addr: Option<String>,
    history_entries: Vec<history_sqlite::HistoryEntry>,
    search_input: String,
    show_config: bool,
    current_preview: String,
    selected_index: Option<usize>,
    hovered_index: Option<usize>,
    window_visible: bool,
    // 分页
    current_page: usize,
    page_size: usize,
}

impl ClipboardShare {
    fn new(
        to_ui_rx: mpsc::UnboundedReceiver<BackendEvent>,
        to_ui_tx: mpsc::UnboundedSender<BackendEvent>,
        p2p_event_tx: mpsc::UnboundedSender<P2PEvent>,
        p2p_event_rx: mpsc::UnboundedReceiver<P2PEvent>,
        clip_tx: mpsc::UnboundedSender<ClipItem>,
        clip_rx: mpsc::UnboundedReceiver<ClipItem>,
    ) -> Self {
        // 启动剪切板监听
        cb::spawn_watcher(clip_tx.clone());

        // 启动全局快捷键监听线程
        hotkey::spawn_global_listener(to_ui_tx.clone());

        // 在独立线程里跑桥接
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(_) => return,
            };
            rt.block_on(async move {
                let mut p2p_rx = p2p_event_rx;
                let mut clip_rx = clip_rx;
                loop {
                    tokio::select! {
                        Some(ev) = p2p_rx.recv() => {
                            let _ = to_ui_tx.send(BackendEvent::P2P(ev));
                        }
                        Some(item) = clip_rx.recv() => {
                            let _ = to_ui_tx.send(BackendEvent::ClipboardChanged(item));
                        }
                        else => break,
                    }
                }
            });
        });

        Self {
            to_ui_rx: Arc::new(Mutex::new(Some(to_ui_rx))),
            p2p_event_tx,
            p2p_send: None,
            connection_status: String::from("未连接"),
            host_port_input: String::from("3939"),
            join_addr_input: String::from("127.0.0.1:3939"),
            psk_input: String::new(),
            generated_psk: None,
            is_hosting: false,
            is_joining: false,
            remote_addr: None,
            history_entries: history_sqlite::load(),
            search_input: String::new(),
            show_config: false,
            current_preview: String::new(),
            selected_index: None,
            hovered_index: None,
            window_visible: true,
            current_page: 0,
            page_size: 9,
        }
    }

    fn update(&mut self, ctx: &egui::Context) {
        // 处理后台事件
        let events = {
            if let Some(rx) = self.to_ui_rx.lock().unwrap().as_mut() {
                let mut evs = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    evs.push(event);
                }
                evs
            } else {
                Vec::new()
            }
        };

        for event in events {
            match event {
                BackendEvent::P2P(ev) => self.handle_p2p_event(ev),
                BackendEvent::ClipboardChanged(item) => self.handle_clipboard_changed(item),
                BackendEvent::ToggleWindow => {
                    self.window_visible = !self.window_visible;
                }
                BackendEvent::QuickCopy(idx) => {
                    if let Some(entry) = self.history_entries.get(idx) {
                        let _ = cb::set(&entry.item);
                    }
                }
            }
        }

        if self.show_config {
            self.show_config_view(ctx);
        } else {
            self.show_history_view(ctx);
        }
    }

    fn handle_p2p_event(&mut self, event: P2PEvent) {
        match event {
            P2PEvent::Connected => {
                self.is_joining = false;
                self.is_hosting = false;
                let addr_str = self.remote_addr.as_ref().map(|s| s.as_str()).unwrap_or("");
                self.connection_status = if addr_str.is_empty() {
                    "已连接".to_string()
                } else {
                    format!("已连接 to {}", addr_str)
                };
            }
            P2PEvent::Disconnected(reason) => {
                self.is_hosting = false;
                self.is_joining = false;
                self.connection_status = format!("已断开: {}", reason);
            }
            P2PEvent::Received(item) => {
                history_sqlite::add(item.clone());
                let _ = cb::set(&item);
                self.current_preview = item.text_preview(80);
                self.history_entries = history_sqlite::load();
            }
            P2PEvent::PskGenerated(psk) => {
                self.generated_psk = Some(psk.clone());
                self.connection_status = format!("PSK: {} (复制此密钥给 Join 方)", psk);
            }
        }
    }

    fn handle_clipboard_changed(&mut self, item: ClipItem) {
        if item.is_image_too_large() {
            self.connection_status = String::from("图片过大（最大 4MB），不同步");
            return;
        }
        history_sqlite::add(item.clone());
        if let Some(ref send) = self.p2p_send {
            let _ = send.send(item.clone());
        }
        self.current_preview = item.text_preview(80);
        self.history_entries = history_sqlite::load();
    }

    fn show_history_view(&mut self, ctx: &egui::Context) {
        // 键盘快捷键
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if let Some(idx) = self.selected_index {
                if idx > 0 {
                    self.selected_index = Some(idx - 1);
                }
            } else if !self.history_entries.is_empty() {
                self.selected_index = Some(self.history_entries.len() - 1);
            }
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if let Some(idx) = self.selected_index {
                if idx + 1 < self.history_entries.len() {
                    self.selected_index = Some(idx + 1);
                }
            } else if !self.history_entries.is_empty() {
                self.selected_index = Some(0);
            }
            ctx.request_repaint();
        }

        // PageUp 上一页
        if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
            if self.current_page > 0 {
                self.current_page -= 1;
                self.selected_index = None;
            }
            ctx.request_repaint();
        }

        // PageDown 下一页
        if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
            let total_pages = if self.history_entries.is_empty() { 1 } else { (self.history_entries.len() + self.page_size - 1) / self.page_size };
            if self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.selected_index = None;
            }
            ctx.request_repaint();
        }

        // Enter键应用选中项
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(idx) = self.selected_index {
                if let Some(entry) = self.history_entries.get(idx) {
                    let _ = cb::set(&entry.item);
                    if let Some(ref send) = self.p2p_send {
                        let _ = send.send(entry.item.clone());
                    }
                }
            }
        }

        // 数字键 1-9 快速选择并应用（当前页）
        let start_idx = self.current_page * self.page_size;
        let num_keys = [
            (egui::Key::Num1, 0),
            (egui::Key::Num2, 1),
            (egui::Key::Num3, 2),
            (egui::Key::Num4, 3),
            (egui::Key::Num5, 4),
            (egui::Key::Num6, 5),
            (egui::Key::Num7, 6),
            (egui::Key::Num8, 7),
            (egui::Key::Num9, 8),
        ];
        for (key, local_idx) in num_keys {
            if ctx.input(|i| i.key_pressed(key)) {
                let global_idx = start_idx + local_idx;
                if let Some(entry) = self.history_entries.get(global_idx) {
                    self.selected_index = Some(global_idx);
                    let _ = cb::set(&entry.item);
                    if let Some(ref send) = self.p2p_send {
                        let _ = send.send(entry.item.clone());
                    }
                }
                break;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(460.0);

            // 顶部标题栏
            egui::Frame::none()
                .fill(egui::Color32::from_gray(40))
                .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new("📋 剪切板共享").color(egui::Color32::WHITE).size(20.0));
                        ui.separator();
                        let is_connected = self.connection_status.contains("已连接");
                        let indicator = if is_connected { "🟢" } else { "⚪" };
                        ui.label(egui::RichText::new(format!("{} {}", indicator, self.connection_status)).color(egui::Color32::WHITE).size(14.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("⚙️ 配置").size(14.0))
                                    .fill(egui::Color32::from_gray(70))
                                    .rounding(6.0)
                            ).clicked() {
                                self.show_config = true;
                            }
                        });
                    });
                });

            ui.add_space(15.0);

            // 当前内容预览
            egui::Frame::none()
                .fill(egui::Color32::from_gray(245))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(200)))
                .rounding(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("📄 当前内容").size(14.0).strong().color(egui::Color32::from_gray(80)));
                    ui.add_space(8.0);
                    let preview = if self.current_preview.is_empty() {
                        "暂无内容".to_string()
                    } else {
                        self.current_preview.clone()
                    };
                    ui.label(egui::RichText::new(preview).size(15.0));
                });

            ui.add_space(15.0);

            // 搜索框
            egui::Frame::none()
                .fill(egui::Color32::from_gray(250))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(200)))
                .rounding(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔍").size(18.0));
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_input)
                                .hint_text("搜索历史记录...")
                                .desired_width(f32::INFINITY - 40.0)
                                .frame(false)
                        );
                        if response.changed() {
                            self.history_entries = history_sqlite::search(&self.search_input);
                            self.current_page = 0;
                        }
                    });
                });

            ui.add_space(10.0);

            // 分页导航
            let total_entries = self.history_entries.len();
            let total_pages = if total_entries == 0 { 1 } else { (total_entries + self.page_size - 1) / self.page_size };
            if self.current_page >= total_pages {
                self.current_page = if total_pages > 0 { total_pages - 1 } else { 0 };
            }

            ui.horizontal(|ui| {
                ui.set_enabled(self.current_page > 0);
                if ui.add(egui::Button::new("◀").small()).clicked() {
                    if self.current_page > 0 {
                        self.current_page -= 1;
                        self.selected_index = None;
                    }
                }
                ui.set_enabled(true);
                ui.label(format!("第 {} / {} 页", self.current_page + 1, total_pages));
                ui.set_enabled(self.current_page + 1 < total_pages);
                if ui.add(egui::Button::new("▶").small()).clicked() {
                    if self.current_page + 1 < total_pages {
                        self.current_page += 1;
                        self.selected_index = None;
                    }
                }
                ui.set_enabled(true);
                ui.add_space(10.0);
                ui.label("每页:");
                egui::ComboBox::from_id_salt("page_size")
                    .selected_text(format!("{}", self.page_size))
                    .show_ui(ui, |ui| {
                        for size in [9, 18, 27, 36].iter() {
                            ui.selectable_value(&mut self.page_size, *size, format!("{}", size));
                        }
                    });
            });

            ui.add_space(10.0);

            // 历史列表
            let start_idx = self.current_page * self.page_size;
            let end_idx = (start_idx + self.page_size).min(total_entries);

            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    let mut hover_idx = None;
                    let mut click_idx = None;
                    let mut click_item = None;
                    let full_width = ui.available_width();
                    let entries_to_remove = {
                        let mut ids = Vec::new();
                        for local_i in start_idx..end_idx {
                            let entry = &self.history_entries[local_i];
                            let preview = entry.item.text_preview(80);
                            let item_id = entry.id;
                            let item_clone = entry.item.clone();
                            let is_selected = self.selected_index == Some(local_i);
                            let is_hovered = self.hovered_index == Some(local_i);

                            let (bg_color, border_color) = if is_selected {
                                (egui::Color32::from_rgb(220, 235, 255), egui::Color32::from_rgb(100, 150, 220))
                            } else if is_hovered {
                                (egui::Color32::from_gray(240), egui::Color32::from_gray(180))
                            } else {
                                (egui::Color32::WHITE, egui::Color32::from_gray(220))
                            };

                            let item_resp = egui::Frame::none()
                                .fill(bg_color)
                                .stroke(egui::Stroke::new(is_selected.then_some(2.0).unwrap_or(1.0), border_color))
                                .rounding(6.0)
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    // 强制占满宽度和固定高度
                                    ui.set_min_size(egui::vec2(full_width, 45.0));
                                    ui.set_max_size(egui::vec2(full_width, 45.0));

                                    ui.horizontal(|ui| {
                                        // 序号
                                        ui.add_sized([28.0, 30.0], egui::Label::new(
                                            egui::RichText::new(format!("{}", (local_i % 9) + 1))
                                                .size(14.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(100, 100, 100))
                                        ));

                                        ui.add_space(8.0);

                                        // 内容区域
                                        let resp = ui.label(egui::RichText::new(&preview).size(14.0));
                                        if resp.hovered() {
                                            hover_idx = Some(local_i);
                                        }
                                        if resp.clicked() {
                                            click_idx = Some(local_i);
                                            click_item = Some(item_clone);
                                        }

                                        let truncated_at = if entry.at.len() > 20 {
                                            format!("{}...", &entry.at[..17])
                                        } else {
                                            entry.at.clone()
                                        };
                                        ui.label(egui::RichText::new(truncated_at).size(11.0).color(egui::Color32::GRAY));

                                        // 删除按钮靠右
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.add(
                                                egui::Button::new(egui::RichText::new("🗑️").size(14.0))
                                                    .fill(egui::Color32::TRANSPARENT)
                                                    .small()
                                            ).clicked() {
                                                ids.push(item_id);
                                            }
                                        });
                                    });
                                }).response;
                                // 如果是选中项，自动滚动到可见区域
                                if is_selected {
                                    ui.scroll_to_rect(item_resp.rect, Some(egui::Align::Center));
                                }
                            ui.add_space(5.0);
                        }

                        if let Some(h) = hover_idx {
                            self.hovered_index = Some(h);
                        }
                        if let (Some(idx), Some(item)) = (click_idx, click_item) {
                            self.selected_index = Some(idx);
                            let _ = cb::set(&item);
                            if let Some(ref send) = self.p2p_send {
                                let _ = send.send(item.clone());
                            }
                        }
                        ids
                    };

                    for id in entries_to_remove {
                        history_sqlite::delete(id);
                        self.history_entries = history_sqlite::search(&self.search_input);
                        if let Some(idx) = self.selected_index {
                            if idx >= self.history_entries.len() {
                                self.selected_index = self.history_entries.len().checked_sub(1);
                            }
                        }
                        let new_total = self.history_entries.len();
                        let new_pages = if new_total == 0 { 1 } else { (new_total + self.page_size - 1) / self.page_size };
                        if self.current_page >= new_pages {
                            self.current_page = new_pages.saturating_sub(1);
                        }
                    }
                });
        });
    }

    fn show_config_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading("⚙️ P2P 配置");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("← 返回").clicked() {
                        self.show_config = false;
                    }
                });
            });
            ui.add_space(20.0);

            // Host 配置卡片
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(200)))
                .rounding(8.0)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("🖥️ 作为主机 (Host)").size(16.0).strong());
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("监听端口:");
                        ui.add(egui::TextEdit::singleline(&mut self.host_port_input).desired_width(100.0));
                        ui.add_space(10.0);
                        if self.is_hosting {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("⏹ 停止监听").size(14.0))
                                    .rounding(6.0)
                                    .fill(egui::Color32::from_rgb(220, 53, 69))
                            ).clicked() {
                                self.p2p_send = None;
                                self.is_hosting = false;
                                self.generated_psk = None;
                                self.connection_status = String::from("已停止监听");
                            }
                        } else {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("▶ 开始监听").size(14.0))
                                    .rounding(6.0)
                                    .fill(egui::Color32::from_rgb(40, 167, 69))
                            ).clicked() {
                                if let Some(port) = self.host_port_input.parse().ok() {
                                    if (1..=65535).contains(&port) {
                                        let tx = self.p2p_event_tx.clone();
                                        self.p2p_send = Some(p2p::run_host(port, tx));
                                        self.is_hosting = true;
                                        self.connection_status = format!("监听 0.0.0.0:{} …", port);
                                    } else {
                                        self.connection_status = String::from("端口号必须在 1-65535 范围内");
                                    }
                                }
                            }
                        }
                    });

                    if let Some(ref psk) = self.generated_psk {
                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("🔐 分享此密钥给连接方:").size(14.0));
                        ui.add_space(5.0);

                        let psk_clone = psk.clone();
                        egui::Frame::none()
                            .fill(egui::Color32::from_gray(245))
                            .rounding(4.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&psk_clone).family(egui::FontFamily::Monospace).size(13.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("📋 复制").clicked() {
                                            let _ = cb::set(&crate::types::ClipItem::Text(psk_clone.clone()));
                                            self.connection_status = String::from("PSK 已复制到剪贴板");
                                        }
                                    });
                                });
                            });
                    }
                });

            ui.add_space(20.0);

            // Join 配置卡片
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(200)))
                .rounding(8.0)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("🔗 连接到主机 (Join)").size(16.0).strong());
                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("主机地址:");
                            ui.add_space(10.0);
                            ui.add(egui::TextEdit::singleline(&mut self.join_addr_input)
                                .hint_text("例如: 192.168.1.100:3939")
                                .desired_width(200.0));
                        });

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label("PSK 密钥:");
                            ui.add_space(10.0);
                            ui.add(egui::TextEdit::singleline(&mut self.psk_input)
                                .hint_text("输入主机提供的密钥")
                                .desired_width(200.0));
                        });

                        ui.add_space(10.0);

                        if self.is_joining {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("正在连接…").color(egui::Color32::GRAY));
                                ui.add_space(10.0);
                                if ui.button("取消").clicked() {
                                    self.p2p_send = None;
                                    self.is_joining = false;
                                    self.remote_addr = None;
                                    self.connection_status = String::from("已取消连接");
                                }
                            });
                        } else {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("🔗 连接").size(14.0))
                                    .rounding(6.0)
                                    .fill(egui::Color32::from_rgb(0, 123, 255))
                            ).clicked() {
                                let addr = self.join_addr_input.clone();
                                let psk = self.psk_input.clone();
                                if !psk.is_empty() {
                                    let tx = self.p2p_event_tx.clone();
                                    self.p2p_send = Some(p2p::run_join(addr.clone(), psk, tx));
                                    self.is_joining = true;
                                    self.remote_addr = Some(addr);
                                    self.connection_status = "正在连接…".to_string();
                                } else {
                                    self.connection_status = String::from("请输入 PSK");
                                }
                            }
                        }
                    });
                });
        });
    }
}

impl eframe::App for ClipboardShare {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_custom_fonts(ctx);
        self.update(ctx);
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    #[cfg(target_os = "macos")]
    {
        if let Ok(font_data) = std::fs::read("/System/Library/AssetsV2/com_apple_MobileAsset_Font8/86ba2c91f017a3749571a82f2c6d890ac7ffb2fb.asset/AssetData/PingFang.ttc") {
            fonts.font_data.insert("PingFangSC".to_owned(), egui::FontData::from_owned(font_data));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "PingFangSC".to_owned());
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert("MicrosoftYaHei".to_owned(), egui::FontData::from_owned(font_data));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "MicrosoftYaHei".to_owned());
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(font_data) = std::fs::read("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc") {
            fonts.font_data.insert("WenQuanYiZenHei".to_owned(), egui::FontData::from_owned(font_data));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "WenQuanYiZenHei".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let (to_ui_tx, to_ui_rx) = mpsc::unbounded_channel();
    let (p2p_event_tx, p2p_event_rx) = mpsc::unbounded_channel();
    let (clip_tx, clip_rx) = mpsc::unbounded_channel();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 750.0])
            .with_min_inner_size([460.0, 600.0])
            .with_max_inner_size([460.0, 900.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native("剪切板共享", options, Box::new(|_cc| {
        let app = ClipboardShare::new(to_ui_rx, to_ui_tx, p2p_event_tx, p2p_event_rx, clip_tx, clip_rx);
        Ok(Box::new(app) as Box<dyn eframe::App>)
    }))
}
