//! P2P 加密通道：Noise 协议 over TCP，双端加密，低延迟

use crate::types::ClipItem;
use snow::{Builder, TransportState};
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

const NOISE_PATTERN: &str = "Noise_NN_25519_ChaChaPoly_BLAKE2s";
const MAX_FRAME: usize = 4 * 1024 * 1024; // 4MB

/// P2P 事件：收到远端剪切板 / 连接状态
#[derive(Debug, Clone)]
pub enum P2PEvent {
    Received(ClipItem),
    Connected,
    Disconnected,
}

async fn do_handshake_initiator(stream: &mut TcpStream) -> io::Result<TransportState> {
    let mut initiator = Builder::new(NOISE_PATTERN.parse().map_err(|e: snow::Error| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?)
    .build_initiator()
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut buf = [0u8; 1024];
    let len = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    stream.write_all(&buf[..len]).await?;

    let mut read_buf = [0u8; 1024];
    let n = stream.read(&mut read_buf).await?;
    initiator
        .read_message(&read_buf[..n], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    initiator.into_transport_mode().map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

async fn do_handshake_responder(stream: &mut TcpStream) -> io::Result<TransportState> {
    let mut responder = Builder::new(NOISE_PATTERN.parse().map_err(|e: snow::Error| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?)
    .build_responder()
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut buf = [0u8; 1024];
    let mut read_buf = [0u8; 1024];
    let n = stream.read(&mut read_buf).await?;
    responder
        .read_message(&read_buf[..n], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = responder
        .write_message(&[], &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    stream.write_all(&buf[..len]).await?;

    responder.into_transport_mode().map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

fn encode_item(item: &ClipItem) -> io::Result<Vec<u8>> {
    bincode::serialize(item).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

fn decode_item(bytes: &[u8]) -> io::Result<ClipItem> {
    bincode::deserialize(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// 在后台运行 Host（监听端口），握手成功后通过 event_tx 上报事件并转发收到的剪切板；
/// 返回的 sender 用于向对端发送剪切板
pub fn run_host(
    port: u16,
    event_tx: mpsc::UnboundedSender<P2PEvent>,
) -> mpsc::UnboundedSender<ClipItem> {
    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<ClipItem>();

    tokio::spawn(async move {
        let listener = match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(l) => l,
            Err(_) => {
                let _ = event_tx.send(P2PEvent::Disconnected);
                return;
            }
        };
        let (mut stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => {
                let _ = event_tx.send(P2PEvent::Disconnected);
                return;
            }
        };
        let transport = match do_handshake_responder(&mut stream).await {
            Ok(t) => t,
            Err(_) => {
                let _ = event_tx.send(P2PEvent::Disconnected);
                return;
            }
        };
        let _ = event_tx.send(P2PEvent::Connected);
        let (mut reader, mut writer) = stream.into_split();
        let transport = Arc::new(Mutex::new(transport));
        let event_tx2 = event_tx.clone();
        let transport_reader = transport.clone();
        tokio::spawn(async move {
            let mut read_len_buf = [0u8; 4];
            let mut read_payload = vec![0u8; MAX_FRAME];
            let mut dec_buf = vec![0u8; MAX_FRAME + 64];
            loop {
                if reader.read_exact(&mut read_len_buf).await.is_err() {
                    break;
                }
                let len = u32::from_le_bytes(read_len_buf) as usize;
                if len > MAX_FRAME {
                    break;
                }
                if reader.read_exact(&mut read_payload[..len]).await.is_err() {
                    break;
                }
                let n = match transport_reader.lock().await.read_message(&read_payload[..len], &mut dec_buf) {
                    Ok(n) => n,
                    Err(_) => break,
                };
                if let Ok(clip) = decode_item(&dec_buf[..n]) {
                    let _ = event_tx2.send(P2PEvent::Received(clip));
                }
            }
        });
        let mut write_buf = vec![0u8; MAX_FRAME + 64];
        loop {
            let item = match send_rx.recv().await {
                Some(i) => i,
                None => break,
            };
            let plain = match encode_item(&item) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let n = match transport.lock().await.write_message(&plain, &mut write_buf) {
                Ok(n) => n,
                Err(_) => break,
            };
            let len_bytes = (n as u32).to_le_bytes();
            if writer.write_all(&len_bytes).await.is_err() {
                break;
            }
            if writer.write_all(&write_buf[..n]).await.is_err() {
                break;
            }
        }
        let _ = event_tx.send(P2PEvent::Disconnected);
    });

    send_tx
}

/// 在后台运行 Join（连接对方地址），握手成功后同上
pub fn run_join(
    addr: String,
    event_tx: mpsc::UnboundedSender<P2PEvent>,
) -> mpsc::UnboundedSender<ClipItem> {
    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<ClipItem>();

    tokio::spawn(async move {
        let mut stream = match TcpStream::connect(&addr).await {
            Ok(s) => s,
            Err(_) => {
                let _ = event_tx.send(P2PEvent::Disconnected);
                return;
            }
        };
        let transport = match do_handshake_initiator(&mut stream).await {
            Ok(t) => t,
            Err(_) => {
                let _ = event_tx.send(P2PEvent::Disconnected);
                return;
            }
        };
        let _ = event_tx.send(P2PEvent::Connected);
        let (mut reader, mut writer) = stream.into_split();
        let transport = Arc::new(Mutex::new(transport));
        let event_tx2 = event_tx.clone();
        let transport_reader = transport.clone();
        tokio::spawn(async move {
            let mut read_len_buf = [0u8; 4];
            let mut read_payload = vec![0u8; MAX_FRAME];
            let mut dec_buf = vec![0u8; MAX_FRAME + 64];
            loop {
                if reader.read_exact(&mut read_len_buf).await.is_err() {
                    break;
                }
                let len = u32::from_le_bytes(read_len_buf) as usize;
                if len > MAX_FRAME {
                    break;
                }
                if reader.read_exact(&mut read_payload[..len]).await.is_err() {
                    break;
                }
                let n = match transport_reader.lock().await.read_message(&read_payload[..len], &mut dec_buf) {
                    Ok(n) => n,
                    Err(_) => break,
                };
                if let Ok(clip) = decode_item(&dec_buf[..n]) {
                    let _ = event_tx2.send(P2PEvent::Received(clip));
                }
            }
        });
        let mut write_buf = vec![0u8; MAX_FRAME + 64];
        loop {
            let item = match send_rx.recv().await {
                Some(i) => i,
                None => break,
            };
            let plain = match encode_item(&item) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let n = match transport.lock().await.write_message(&plain, &mut write_buf) {
                Ok(n) => n,
                Err(_) => break,
            };
            let len_bytes = (n as u32).to_le_bytes();
            if writer.write_all(&len_bytes).await.is_err() {
                break;
            }
            if writer.write_all(&write_buf[..n]).await.is_err() {
                break;
            }
        }
        let _ = event_tx.send(P2PEvent::Disconnected);
    });

    send_tx
}
