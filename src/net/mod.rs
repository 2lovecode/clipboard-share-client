//! Peer WebSocket listen, dial, auth, and session arbitration.

use crate::protocol::{decode, encode, Envelope, MessageKind};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use warp::ws::{Message as WsMessage, WebSocket};
use warp::Filter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnRole {
    Dialer,
    Listener,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    AuthFailed,
    Error,
}

/// Keep the connection where the lexicographically smaller peer_id is the dialer.
pub fn should_keep_connection(
    local_peer_id: &str,
    remote_peer_id: &str,
    local_role: ConnRole,
) -> bool {
    let local_should_dial = local_peer_id < remote_peer_id;
    match local_role {
        ConnRole::Dialer => local_should_dial,
        ConnRole::Listener => !local_should_dial,
    }
}

pub fn passphrases_match(expected: &str, provided: &str) -> bool {
    let a = expected.as_bytes();
    let b = provided.as_bytes();
    if a.len() != b.len() {
        let _ = a.ct_eq(a);
        return false;
    }
    bool::from(a.ct_eq(b))
}

pub fn make_auth_envelope(local_peer_id: &str, passphrase: &str) -> Envelope {
    Envelope {
        v: 1,
        kind: MessageKind::Auth,
        id: Uuid::new_v4().to_string(),
        ts: chrono::Utc::now().timestamp_millis(),
        peer_id: Some(local_peer_id.to_string()),
        passphrase: Some(passphrase.to_string()),
        payload: None,
    }
}

pub fn respond_to_auth(expected_passphrase: &str, incoming: &Envelope) -> Envelope {
    let ok = incoming
        .passphrase
        .as_deref()
        .map(|p| passphrases_match(expected_passphrase, p))
        .unwrap_or(false);
    Envelope {
        v: 1,
        kind: if ok {
            MessageKind::AuthOk
        } else {
            MessageKind::AuthFail
        },
        id: Uuid::new_v4().to_string(),
        ts: chrono::Utc::now().timestamp_millis(),
        peer_id: None,
        passphrase: None,
        payload: None,
    }
}

#[derive(Debug, Clone)]
pub struct PeerEvent {
    pub envelope: Envelope,
}

#[derive(Clone)]
pub struct OutboundHandle {
    tx: mpsc::UnboundedSender<Envelope>,
}

impl OutboundHandle {
    pub fn send(&self, env: Envelope) -> Result<(), String> {
        self.tx.send(env).map_err(|e| e.to_string())
    }
}

pub struct SessionHub {
    pub local_peer_id: String,
    pub passphrase: Mutex<String>,
    state: Mutex<ConnectionState>,
    inbound_tx: mpsc::UnboundedSender<PeerEvent>,
    outbound: Mutex<Option<OutboundHandle>>,
    remote_peer_id: Mutex<Option<String>>,
}

impl SessionHub {
    pub fn new(
        local_peer_id: String,
        passphrase: String,
    ) -> (Arc<Self>, mpsc::UnboundedReceiver<PeerEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Arc::new(Self {
                local_peer_id,
                passphrase: Mutex::new(passphrase),
                state: Mutex::new(ConnectionState::Disconnected),
                inbound_tx: tx,
                outbound: Mutex::new(None),
                remote_peer_id: Mutex::new(None),
            }),
            rx,
        )
    }

    pub async fn set_passphrase(&self, pw: String) {
        *self.passphrase.lock().await = pw;
    }

    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    async fn set_state(&self, s: ConnectionState) {
        *self.state.lock().await = s;
    }

    pub async fn outbound(&self) -> Option<OutboundHandle> {
        self.outbound.lock().await.clone()
    }

    async fn attach_session(
        &self,
        remote_peer_id: Option<String>,
        role: ConnRole,
        handle: OutboundHandle,
    ) {
        if let Some(remote) = remote_peer_id.as_ref() {
            if let Some(existing) = self.outbound.lock().await.as_ref() {
                let _ = existing; // presence means duplicate path
                if !should_keep_connection(&self.local_peer_id, remote, role) {
                    return;
                }
            }
            *self.remote_peer_id.lock().await = Some(remote.clone());
        }
        *self.outbound.lock().await = Some(handle);
        self.set_state(ConnectionState::Connected).await;
    }

    pub async fn send_envelope(&self, env: Envelope) -> Result<(), String> {
        let out = self
            .outbound
            .lock()
            .await
            .clone()
            .ok_or_else(|| "not connected".to_string())?;
        out.send(env)
    }
}

/// Start listening for inbound WebSocket peers on `port`.
pub async fn start_listener(hub: Arc<SessionHub>, port: u16) -> Result<(), String> {
    let hub_filter = {
        let hub = hub.clone();
        warp::any().map(move || hub.clone())
    };
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(hub_filter)
        .map(|ws: warp::ws::Ws, hub: Arc<SessionHub>| {
            ws.on_upgrade(move |socket| handle_inbound(socket, hub))
        });
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    tokio::spawn(async move {
        warp::serve(ws_route).run(addr).await;
    });
    Ok(())
}

async fn handle_inbound(socket: WebSocket, hub: Arc<SessionHub>) {
    hub.set_state(ConnectionState::Connecting).await;
    let (mut tx, mut rx) = socket.split();
    let first = match rx.next().await {
        Some(Ok(msg)) if msg.is_text() => msg.to_str().unwrap_or("").to_string(),
        _ => {
            hub.set_state(ConnectionState::Error).await;
            return;
        }
    };
    let env = match decode(&first) {
        Ok(e) => e,
        Err(_) => {
            hub.set_state(ConnectionState::Error).await;
            return;
        }
    };
    let remote_id = env.peer_id.clone();
    let passphrase = hub.passphrase.lock().await.clone();
    let reply = respond_to_auth(&passphrase, &env);
    let raw = match encode(&reply) {
        Ok(r) => r,
        Err(_) => {
            hub.set_state(ConnectionState::Error).await;
            return;
        }
    };
    if tx.send(WsMessage::text(raw)).await.is_err() {
        hub.set_state(ConnectionState::Error).await;
        return;
    }
    if reply.kind == MessageKind::AuthFail {
        hub.set_state(ConnectionState::AuthFailed).await;
        return;
    }

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Envelope>();
    hub.attach_session(
        remote_id,
        ConnRole::Listener,
        OutboundHandle { tx: out_tx },
    )
    .await;

    loop {
        tokio::select! {
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(msg)) if msg.is_text() => {
                        if let Ok(env) = decode(msg.to_str().unwrap_or("")) {
                            let _ = hub.inbound_tx.send(PeerEvent { envelope: env });
                        }
                    }
                    Some(Ok(_)) => {}
                    _ => break,
                }
            }
            maybe = out_rx.recv() => {
                match maybe {
                    Some(env) => {
                        if let Ok(raw) = encode(&env) {
                            if tx.send(WsMessage::text(raw)).await.is_err() {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        }
    }
    *hub.outbound.lock().await = None;
    hub.set_state(ConnectionState::Disconnected).await;
}

/// Dial a peer at host:port and complete auth as dialer; keep session alive.
pub async fn dial_peer(hub: Arc<SessionHub>, host: &str, port: u16) -> Result<(), String> {
    hub.set_state(ConnectionState::Connecting).await;
    let url = format!("ws://{}:{}/ws", host, port);
    let (ws, _) = async_tungstenite::tokio::connect_async(&url)
        .await
        .map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();
    let auth = make_auth_envelope(&hub.local_peer_id, &hub.passphrase.lock().await);
    let raw = encode(&auth).map_err(|e| e.to_string())?;
    write
        .send(async_tungstenite::tungstenite::Message::Text(raw))
        .await
        .map_err(|e| e.to_string())?;
    let resp = read
        .next()
        .await
        .ok_or_else(|| "connection closed before auth".to_string())?
        .map_err(|e| e.to_string())?;
    let text = match resp {
        async_tungstenite::tungstenite::Message::Text(t) => t,
        _ => return Err("expected text auth response".into()),
    };
    let env = decode(&text)?;
    match env.kind {
        MessageKind::AuthOk => {}
        MessageKind::AuthFail => {
            hub.set_state(ConnectionState::AuthFailed).await;
            return Err("authentication failed".into());
        }
        _ => {
            hub.set_state(ConnectionState::Error).await;
            return Err("unexpected auth response".into());
        }
    }

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Envelope>();
    hub.attach_session(None, ConnRole::Dialer, OutboundHandle { tx: out_tx })
        .await;

    let hub_bg = hub.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                incoming = read.next() => {
                    match incoming {
                        Some(Ok(async_tungstenite::tungstenite::Message::Text(t))) => {
                            if let Ok(env) = decode(&t) {
                                let _ = hub_bg.inbound_tx.send(PeerEvent { envelope: env });
                            }
                        }
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
                maybe = out_rx.recv() => {
                    match maybe {
                        Some(env) => {
                            if let Ok(raw) = encode(&env) {
                                if write
                                    .send(async_tungstenite::tungstenite::Message::Text(raw))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                        None => break,
                    }
                }
            }
        }
        *hub_bg.outbound.lock().await = None;
        hub_bg.set_state(ConnectionState::Disconnected).await;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{MessageKind, Payload};

    #[test]
    fn passphrase_match_ok() {
        assert!(passphrases_match("secret", "secret"));
        assert!(!passphrases_match("secret", "wrong"));
    }

    #[test]
    fn auth_response_ok_and_fail() {
        let ok_env = make_auth_envelope("peer-a", "pw");
        let resp = respond_to_auth("pw", &ok_env);
        assert_eq!(resp.kind, MessageKind::AuthOk);

        let bad = make_auth_envelope("peer-a", "nope");
        let resp = respond_to_auth("pw", &bad);
        assert_eq!(resp.kind, MessageKind::AuthFail);
    }

    #[test]
    fn dual_connect_keeps_smaller_id_as_dialer() {
        assert!(should_keep_connection("a", "b", ConnRole::Dialer));
        assert!(!should_keep_connection("a", "b", ConnRole::Listener));
        assert!(!should_keep_connection("b", "a", ConnRole::Dialer));
        assert!(should_keep_connection("b", "a", ConnRole::Listener));
    }

    #[tokio::test]
    async fn loopback_manual_ip_auth_success() {
        let (hub_server, _) = SessionHub::new("server".into(), "shared".into());
        start_listener(hub_server.clone(), 17890)
            .await
            .expect("listen");
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let (hub_client, _) = SessionHub::new("client".into(), "shared".into());
        dial_peer(hub_client.clone(), "127.0.0.1", 17890)
            .await
            .expect("dial");
        assert_eq!(hub_client.state().await, ConnectionState::Connected);
    }

    #[tokio::test]
    async fn loopback_wrong_passphrase_fails() {
        let (hub_server, _) = SessionHub::new("server".into(), "shared".into());
        start_listener(hub_server.clone(), 17891)
            .await
            .expect("listen");
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let (hub_client, _) = SessionHub::new("client".into(), "wrong".into());
        let err = dial_peer(hub_client.clone(), "127.0.0.1", 17891)
            .await
            .expect_err("should fail");
        assert!(err.contains("auth") || err.contains("fail"));
        assert_eq!(hub_client.state().await, ConnectionState::AuthFailed);
    }

    #[tokio::test]
    async fn loopback_history_item_round_trip() {
        let (hub_server, mut server_rx) = SessionHub::new("server".into(), "shared".into());
        start_listener(hub_server.clone(), 17892)
            .await
            .expect("listen");
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let (hub_client, _) = SessionHub::new("client".into(), "shared".into());
        dial_peer(hub_client.clone(), "127.0.0.1", 17892)
            .await
            .expect("dial");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let env = Envelope {
            v: 1,
            kind: MessageKind::HistoryItem,
            id: "h1".into(),
            ts: 1,
            peer_id: None,
            passphrase: None,
            payload: Some(Payload::Text {
                text: "ping".into(),
            }),
        };
        hub_client.send_envelope(env).await.expect("send");
        let got = tokio::time::timeout(std::time::Duration::from_secs(2), server_rx.recv())
            .await
            .expect("timeout")
            .expect("event");
        assert_eq!(got.envelope.kind, MessageKind::HistoryItem);
        match got.envelope.payload {
            Some(Payload::Text { text }) => assert_eq!(text, "ping"),
            _ => panic!("expected text"),
        }
    }
}
