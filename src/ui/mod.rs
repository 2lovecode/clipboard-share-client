//! iced application UI.

use crate::clipboard::{ClipboardPort, SystemClipboard};
use crate::discovery::{can_manual_connect, DiscoveryService, DiscoveredPeer};
use crate::history::{HistoryStore, Source};
use crate::net::{dial_peer, start_listener, ConnectionState, PeerEvent, SessionHub};
use crate::protocol::{Envelope, MessageKind};
use iced::executor;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Application, Color, Command, Element, Length, Subscription, Theme};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct AppFlags {
    pub peer_id: String,
}

impl Default for AppFlags {
    fn default() -> Self {
        Self {
            peer_id: Uuid::new_v4().to_string(),
        }
    }
}

pub struct ClipboardShareApp {
    peer_id: String,
    display_name: String,
    passphrase: String,
    listen_port: String,
    manual_host: String,
    manual_port: String,
    status: ConnectionState,
    status_note: String,
    history: HistoryStore,
    discovered: Vec<DiscoveredPeer>,
    hub: Option<Arc<SessionHub>>,
    event_rx: Option<mpsc::UnboundedReceiver<PeerEvent>>,
    discovery: Option<DiscoveryService>,
    listening: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    DisplayNameChanged(String),
    PassphraseChanged(String),
    ListenPortChanged(String),
    ManualHostChanged(String),
    ManualPortChanged(String),
    StartListen,
    ConnectManual,
    ConnectPeer(usize),
    RefreshDiscovery,
    PushClipboard,
    SelectHistory(usize),
    Tick,
    ListenStarted(Result<(), String>),
    DialFinished(Result<(), String>),
    Status(ConnectionState, String),
    Inbound(Envelope),
    DiscoveryUpdated(Vec<DiscoveredPeer>),
    None,
}

impl Application for ClipboardShareApp {
    type Message = Message;
    type Theme = Theme;
    type Flags = AppFlags;
    type Executor = executor::Default;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let discovery = DiscoveryService::new().ok();
        if let Some(d) = discovery.as_ref() {
            let _ = d.start_browse();
        }
        (
            Self {
                peer_id: flags.peer_id,
                display_name: "clipboard-peer".into(),
                passphrase: "changeme".into(),
                listen_port: "9876".into(),
                manual_host: "127.0.0.1".into(),
                manual_port: "9876".into(),
                status: ConnectionState::Disconnected,
                status_note: String::new(),
                history: HistoryStore::new(100),
                discovered: Vec::new(),
                hub: None,
                event_rx: None,
                discovery,
                listening: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("剪切板共享")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::DisplayNameChanged(v) => self.display_name = v,
            Message::PassphraseChanged(v) => self.passphrase = v,
            Message::ListenPortChanged(v) => self.listen_port = v,
            Message::ManualHostChanged(v) => self.manual_host = v,
            Message::ManualPortChanged(v) => self.manual_port = v,
            Message::RefreshDiscovery => {
                if let Some(d) = self.discovery.as_ref() {
                    self.discovered = d.peers();
                    if let Some(err) = d.last_error() {
                        self.status_note = err;
                    }
                } else {
                    self.status_note = "mDNS 不可用，请使用手动 IP".into();
                }
                let _ = can_manual_connect(self.discovered.len());
            }
            Message::StartListen => {
                let port: u16 = match self.listen_port.parse() {
                    Ok(p) => p,
                    Err(_) => {
                        self.status_note = "端口无效".into();
                        return Command::none();
                    }
                };
                let (hub, rx) = SessionHub::new(self.peer_id.clone(), self.passphrase.clone());
                self.hub = Some(hub.clone());
                self.event_rx = Some(rx);
                self.listening = true;
                if let Some(d) = self.discovery.as_ref() {
                    let host = format!("{}.local.", self.display_name.replace(' ', "-"));
                    let _ = d.advertise(&self.display_name, &host, port, &self.display_name);
                }
                return Command::perform(async move { start_listener(hub, port).await }, |r| {
                    Message::ListenStarted(r)
                });
            }
            Message::ListenStarted(result) => match result {
                Ok(()) => {
                    self.status_note = "已开始监听".into();
                    self.status = ConnectionState::Disconnected;
                }
                Err(e) => self.status_note = e,
            },
            Message::ConnectManual => {
                let host = self.manual_host.clone();
                let port: u16 = match self.manual_port.parse() {
                    Ok(p) => p,
                    Err(_) => {
                        self.status_note = "手动端口无效".into();
                        return Command::none();
                    }
                };
                return self.dial(host, port);
            }
            Message::ConnectPeer(idx) => {
                if let Some(peer) = self.discovered.get(idx).cloned() {
                    return self.dial(peer.host, peer.port);
                }
            }
            Message::DialFinished(result) => match result {
                Ok(()) => {
                    self.status = ConnectionState::Connected;
                    self.status_note = "已连接".into();
                }
                Err(e) => {
                    self.status = ConnectionState::AuthFailed;
                    self.status_note = e;
                }
            },
            Message::PushClipboard => {
                return self.push_clipboard();
            }
            Message::SelectHistory(idx) => {
                // Click writes local clipboard only — no outbound.
                if let Some(item) = self.history.select_for_local_clipboard(idx).cloned() {
                    match SystemClipboard::new() {
                        Ok(mut clip) => {
                            if let Err(e) = clip.write_payload(&item.payload) {
                                self.status_note = e.to_string();
                            } else {
                                self.status_note = "已写入本机剪贴板".into();
                            }
                        }
                        Err(e) => self.status_note = e.to_string(),
                    }
                }
            }
            Message::Inbound(env) => {
                if env.kind == MessageKind::HistoryItem {
                    if let Some(payload) = env.payload {
                        self.history.receive_remote(env.id, payload);
                    }
                }
            }
            Message::Tick => {
                if crate::hotkey_signal::take_push_requested() {
                    return self.push_clipboard();
                }
                if let Some(rx) = self.event_rx.as_mut() {
                    while let Ok(ev) = rx.try_recv() {
                        if ev.envelope.kind == MessageKind::HistoryItem {
                            if let Some(payload) = ev.envelope.payload.clone() {
                                self.history.receive_remote(ev.envelope.id.clone(), payload);
                            }
                        }
                    }
                }
                if let Some(hub) = self.hub.clone() {
                    return Command::perform(async move { hub.state().await }, |s| {
                        Message::Status(s, String::new())
                    });
                }
            }
            Message::Status(s, note) => {
                self.status = s;
                if !note.is_empty() {
                    self.status_note = note;
                }
            }
            Message::DiscoveryUpdated(list) => self.discovered = list,
            Message::None => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(400)).map(|_| Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let status_label = match self.status {
            ConnectionState::Disconnected => "未连接",
            ConnectionState::Connecting => "连接中",
            ConnectionState::Connected => "已连接",
            ConnectionState::AuthFailed => "口令失败",
            ConnectionState::Error => "错误",
        };

        let config = column![
            text(format!("状态: {}  {}", status_label, self.status_note)),
            text_input("显示名", &self.display_name, Message::DisplayNameChanged),
            text_input("共享口令", &self.passphrase, Message::PassphraseChanged),
            text_input("监听端口", &self.listen_port, Message::ListenPortChanged),
            button("开始监听 / 广播").on_press(Message::StartListen),
            Space::with_height(Length::Units(8)),
            text("手动连接（发现失败时仍可用）"),
            text_input("对端 IP", &self.manual_host, Message::ManualHostChanged),
            text_input("对端端口", &self.manual_port, Message::ManualPortChanged),
            row![
                button("连接").on_press(Message::ConnectManual),
                button("刷新发现").on_press(Message::RefreshDiscovery),
                button("推送剪贴板").on_press(Message::PushClipboard),
            ]
            .spacing(8),
        ]
        .spacing(6);

        let peers: Element<_> = if self.discovered.is_empty() {
            text("未发现对端（可用手动 IP）").into()
        } else {
            Column::with_children(
                self.discovered
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        button(text(format!(
                            "{} @ {}:{}",
                            p.display_name, p.host, p.port
                        )))
                        .on_press(Message::ConnectPeer(i))
                        .into()
                    })
                    .collect(),
            )
            .spacing(4)
            .into()
        };

        let history: Element<_> = if self.history.items().is_empty() {
            container(
                text("剪切板历史为空")
                    .style(Color::from_rgb8(0x88, 0x88, 0x88)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
        } else {
            scrollable(
                Column::with_children(
                    self.history
                        .items()
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            let src = match item.source {
                                Source::Local => "本机",
                                Source::Remote => "对端",
                            };
                            button(text(format!("[{}] {}", src, item.summary)))
                                .on_press(Message::SelectHistory(i))
                                .width(Length::Fill)
                                .into()
                        })
                        .collect(),
                )
                .spacing(6)
                .width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        };

        column![config, text("发现的对端"), peers, text("历史（点击写入本机）"), history]
            .padding(16)
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl ClipboardShareApp {
    fn dial(&mut self, host: String, port: u16) -> Command<Message> {
        if self.hub.is_none() {
            let (hub, rx) = SessionHub::new(self.peer_id.clone(), self.passphrase.clone());
            self.hub = Some(hub);
            self.event_rx = Some(rx);
        }
        let hub = self.hub.clone().unwrap();
        let pw = self.passphrase.clone();
        Command::perform(
            async move {
                hub.set_passphrase(pw).await;
                dial_peer(hub, &host, port).await
            },
            Message::DialFinished,
        )
    }

    fn push_clipboard(&mut self) -> Command<Message> {
        let payload = match SystemClipboard::new().and_then(|c| c.read_for_push()) {
            Ok(p) => p,
            Err(e) => {
                self.status_note = e.to_string();
                return Command::none();
            }
        };
        let item = self.history.push_local(payload.clone());
        let connected = matches!(self.status, ConnectionState::Connected);
        if !connected {
            self.status_note = "已入本地历史（未连接，未投递对端）".into();
            return Command::none();
        }
        if let Some(hub) = self.hub.clone() {
            let env = Envelope {
                v: 1,
                kind: MessageKind::HistoryItem,
                id: item.id,
                ts: chrono::Utc::now().timestamp_millis(),
                peer_id: None,
                passphrase: None,
                payload: Some(payload),
            };
            return Command::perform(
                async move {
                    hub.send_envelope(env)
                        .await
                        .map_err(|e| e)
                        .map(|_| ())
                },
                |r| match r {
                    Ok(()) => Message::Status(ConnectionState::Connected, "已推送到对端".into()),
                    Err(e) => Message::Status(ConnectionState::Error, e),
                },
            );
        }
        Command::none()
    }
}

pub fn settings() -> iced::Settings<AppFlags> {
    iced::Settings {
        default_font: font(),
        flags: AppFlags::default(),
        ..Default::default()
    }
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

#[cfg(test)]
mod tests {
    use crate::history::HistoryStore;
    use crate::protocol::Payload;

    #[test]
    fn select_history_does_not_queue_outbound() {
        let mut store = HistoryStore::new(10);
        store.push_local(Payload::Text {
            text: "x".into(),
        });
        let _ = store.take_outbound();
        assert!(store.select_for_local_clipboard(0).is_some());
        assert!(store.peek_outbound().is_empty());
    }
}
