//! 剪切板共享客户端：P2P 加密同步 + 系统剪切板接管 + 本地历史与快捷键

mod clipboard;
mod history;
mod p2p;
mod types;

use crate::clipboard as cb;
use crate::p2p::P2PEvent;
use crate::types::ClipItem;
use iced::executor;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column,
};
use iced::{Application, Color, Command, Element, Length, Subscription, Theme};
use iced::event::{self, Event};
use iced::keyboard;
use iced::subscription;
use iced_graphics;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// 后台事件：P2P 或剪切板变化
#[derive(Debug, Clone)]
enum BackendEvent {
    P2P(P2PEvent),
    ClipboardChanged(ClipItem),
}

#[derive(Debug, Clone)]
enum Message {
    HostPortInput(String),
    JoinAddrInput(String),
    StartHost,
    StartJoin,
    Backend(BackendEvent),
    SearchInput(String),
    ToggleHistoryOverlay,
    PasteFromHistory(u64),
    None,
}

struct ClipboardShare {
    to_ui_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<BackendEvent>>>>,
    p2p_event_tx: mpsc::UnboundedSender<P2PEvent>,
    p2p_send: Option<mpsc::UnboundedSender<ClipItem>>,
    connection_status: String,
    host_port_input: String,
    join_addr_input: String,
    history_entries: Vec<history::HistoryEntry>,
    search_input: String,
    show_history_overlay: bool,
    current_preview: String,
}

fn build_app(
    to_ui_tx: mpsc::UnboundedSender<BackendEvent>,
    to_ui_rx: mpsc::UnboundedReceiver<BackendEvent>,
    p2p_event_tx: mpsc::UnboundedSender<P2PEvent>,
    p2p_event_rx: mpsc::UnboundedReceiver<P2PEvent>,
    clip_tx: mpsc::UnboundedSender<ClipItem>,
    clip_rx: mpsc::UnboundedReceiver<ClipItem>,
) -> ClipboardShare {
    cb::spawn_watcher(clip_tx);
    // 在独立线程里跑桥接，避免在 iced 启动前调用 tokio::spawn 导致无 runtime
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
    ClipboardShare {
        to_ui_rx: Arc::new(Mutex::new(Some(to_ui_rx))),
        p2p_event_tx,
        p2p_send: None,
        connection_status: String::from("未连接"),
        host_port_input: String::from("3939"),
        join_addr_input: String::from("127.0.0.1:3939"),
        history_entries: history::load(),
        search_input: String::new(),
        show_history_overlay: false,
        current_preview: String::new(),
    }
}

impl Application for ClipboardShare {
    type Message = Message;
    type Theme = Theme;
    type Flags = (
        mpsc::UnboundedSender<BackendEvent>,
        mpsc::UnboundedReceiver<BackendEvent>,
        mpsc::UnboundedSender<P2PEvent>,
        mpsc::UnboundedReceiver<P2PEvent>,
        mpsc::UnboundedSender<ClipItem>,
        mpsc::UnboundedReceiver<ClipItem>,
    );
    type Executor = executor::Default;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (to_ui_tx, to_ui_rx, p2p_event_tx, p2p_event_rx, clip_tx, clip_rx) = flags;
        (
            build_app(
                to_ui_tx,
                to_ui_rx,
                p2p_event_tx,
                p2p_event_rx,
                clip_tx,
                clip_rx,
            ),
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("剪切板共享")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::HostPortInput(s) => {
                self.host_port_input = s;
            }
            Message::JoinAddrInput(s) => {
                self.join_addr_input = s;
            }
            Message::StartHost => {
                let port = self.host_port_input.parse().unwrap_or(3939);
                let tx = self.p2p_event_tx.clone();
                self.p2p_send = Some(p2p::run_host(port, tx));
                self.connection_status = format!("监听 0.0.0.0:{} …", port);
            }
            Message::StartJoin => {
                let addr = self.join_addr_input.clone();
                let tx = self.p2p_event_tx.clone();
                self.p2p_send = Some(p2p::run_join(addr, tx));
                self.connection_status = "正在连接…".to_string();
            }
            Message::Backend(BackendEvent::P2P(ev)) => match ev {
                P2PEvent::Connected => {
                    self.connection_status = "已连接".to_string();
                }
                P2PEvent::Disconnected => {
                    self.connection_status = "未连接".to_string();
                }
                P2PEvent::Received(item) => {
                    history::add(item.clone());
                    let _ = cb::set(&item);
                    self.current_preview = item.text_preview(80);
                    self.history_entries = history::load();
                }
            },
            Message::Backend(BackendEvent::ClipboardChanged(item)) => {
                history::add(item.clone());
                if let Some(ref send) = self.p2p_send {
                    let _ = send.send(item.clone());
                }
                self.current_preview = item.text_preview(80);
                self.history_entries = history::load();
            }
            Message::SearchInput(s) => {
                self.search_input = s.clone();
                self.history_entries = history::search(&s);
            }
            Message::ToggleHistoryOverlay => {
                self.show_history_overlay = !self.show_history_overlay;
                if !self.show_history_overlay {
                    self.history_entries = history::search(&self.search_input);
                }
            }
            Message::PasteFromHistory(id) => {
                if let Some(entry) = self.history_entries.iter().find(|e| e.id == id) {
                    let _ = cb::set(&entry.item);
                    if let Some(ref send) = self.p2p_send {
                        let _ = send.send(entry.item.clone());
                    }
                }
                self.show_history_overlay = false;
            }
            Message::None => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let rx = self.to_ui_rx.clone();
        let backend = subscription::unfold(BackendStreamId, rx, |rx_arc| async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let msg = rx_arc
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|r| r.try_recv().ok())
                .map(Message::Backend);
            (msg, rx_arc)
        });
        let hotkey = subscription::events_with(|event, status| {
            if let event::Status::Ignored = status {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Key1,
                    modifiers: keyboard::Modifiers::ALT | keyboard::Modifiers::SHIFT,
                    ..
                }) = event
                {
                    return Some(Message::ToggleHistoryOverlay);
                }
            }
            None
        });
        Subscription::batch(vec![backend, hotkey])
    }

    fn view(&self) -> Element<Message> {
        let conn = row![
            text_input("端口", &self.host_port_input, Message::HostPortInput)
                .padding(8)
                .width(Length::Units(80)),
            button("作为 Host 监听").on_press(Message::StartHost).padding(8),
            text_input("对方地址", &self.join_addr_input, Message::JoinAddrInput)
                .padding(8)
                .width(Length::Units(180)),
            button("连接对方").on_press(Message::StartJoin).padding(8),
            text(&self.connection_status).size(14).style(Color::from_rgb8(0x66, 0x66, 0x66)),
        ]
        .spacing(10)
        .padding(8);

        let preview = container(
            text(if self.current_preview.is_empty() {
                "当前剪切板内容将显示在这里"
            } else {
                &self.current_preview
            })
            .size(14)
            .style(Color::from_rgb8(0x44, 0x44, 0x44)),
        )
        .padding(12)
        .width(Length::Fill);

        let history_btn = button("历史 (Alt+Shift+V)")
            .on_press(Message::ToggleHistoryOverlay)
            .padding(8);

        let main_col = column![
            conn,
            preview,
            history_btn,
            scrollable(
                Column::with_children(
                    self.history_entries
                        .iter()
                        .take(20)
                        .map(|e| {
                            let id = e.id;
                            let preview = e.item.text_preview(60);
                            button(text(preview).size(12))
                                .on_press(Message::PasteFromHistory(id))
                                .padding(6)
                                .width(Length::Fill)
                        })
                        .map(Element::from)
                        .collect(),
                )
                .spacing(4)
                .width(Length::Fill),
            )
            .height(Length::Units(200)),
        ]
        .spacing(12)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        if self.show_history_overlay {
            let search = text_input("搜索历史", &self.search_input, Message::SearchInput)
                .padding(8)
                .width(Length::Fill);
            let list = scrollable(
                Column::with_children(
                    self.history_entries
                        .iter()
                        .take(50)
                        .map(|e| {
                            let id = e.id;
                            let preview = e.item.text_preview(80);
                            button(text(preview).size(12))
                                .on_press(Message::PasteFromHistory(id))
                                .padding(8)
                                .width(Length::Fill)
                        })
                        .map(Element::from)
                        .collect(),
                )
                .spacing(4)
                .width(Length::Fill),
            )
            .height(Length::Units(300));
            let overlay = container(
                column![
                    search,
                    list,
                    button("关闭").on_press(Message::ToggleHistoryOverlay).padding(8),
                ]
                .spacing(10)
                .padding(20),
            )
            .width(Length::Units(400))
            .padding(20);
            column![main_col, overlay].into()
        } else {
            main_col.into()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BackendStreamId;

pub fn main() -> iced::Result {
    let (to_ui_tx, to_ui_rx) = mpsc::unbounded_channel();
    let (p2p_event_tx, p2p_event_rx) = mpsc::unbounded_channel();
    let (clip_tx, clip_rx) = mpsc::unbounded_channel();
    let mut settings = iced::Settings::with_flags((
        to_ui_tx,
        to_ui_rx,
        p2p_event_tx,
        p2p_event_rx,
        clip_tx,
        clip_rx,
    ));
    settings.default_font = font();
    ClipboardShare::run(settings)
}

static FONT: OnceCell<Option<Vec<u8>>> = OnceCell::new();
fn font() -> Option<&'static [u8]> {
    FONT.get_or_init(|| {
        use iced_graphics::font::Family;
        let source = iced_graphics::font::Source::new();
        source
            .load(&[
                Family::Title("PingFang SC".to_owned()),
                Family::Title("Hiragino Sans GB".to_owned()),
                Family::Title("Heiti SC".to_owned()),
                Family::Title("Microsoft YaHei".to_owned()),
                Family::Title("WenQuanYi Micro Hei".to_owned()),
                Family::Title("Helvetica".to_owned()),
                Family::Title("Tahoma".to_owned()),
                Family::Title("Arial".to_owned()),
                Family::SansSerif,
            ])
            .ok()
    })
    .as_ref()
    .map(|f| f.as_slice())
}
