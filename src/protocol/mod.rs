//! Peer message envelope encode/decode.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Auth,
    AuthOk,
    AuthFail,
    HistoryItem,
    Goodbye,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Text { text: String },
    Html { html: String, plain: Option<String> },
    Image { mime: String, data_base64: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub kind: MessageKind,
    pub id: String,
    pub ts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
}

pub fn encode(envelope: &Envelope) -> Result<String, String> {
    serde_json::to_string(envelope).map_err(|e| e.to_string())
}

pub fn decode(raw: &str) -> Result<Envelope, String> {
    let env: Envelope = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if env.v == 0 {
        return Err("unsupported protocol version".into());
    }
    match env.kind {
        MessageKind::Auth => {
            if env.peer_id.as_ref().map(|s| s.is_empty()).unwrap_or(true)
                || env.passphrase.is_none()
            {
                return Err("auth requires peer_id and passphrase".into());
            }
        }
        MessageKind::HistoryItem => {
            if env.payload.is_none() {
                return Err("history_item requires payload".into());
            }
        }
        MessageKind::AuthOk | MessageKind::AuthFail | MessageKind::Goodbye => {}
    }
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_auth() {
        let env = Envelope {
            v: 1,
            kind: MessageKind::Auth,
            id: "msg-1".into(),
            ts: 100,
            peer_id: Some("peer-a".into()),
            passphrase: Some("secret".into()),
            payload: None,
        };
        let raw = encode(&env).expect("encode");
        let got = decode(&raw).expect("decode");
        assert_eq!(got, env);
    }

    #[test]
    fn encodes_and_decodes_history_item_text() {
        let env = Envelope {
            v: 1,
            kind: MessageKind::HistoryItem,
            id: "msg-2".into(),
            ts: 200,
            peer_id: None,
            passphrase: None,
            payload: Some(Payload::Text {
                text: "hello".into(),
            }),
        };
        let raw = encode(&env).expect("encode");
        let got = decode(&raw).expect("decode");
        assert_eq!(got, env);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = decode("not-json").expect_err("should fail");
        assert!(!err.is_empty());
    }

    #[test]
    fn rejects_history_item_without_payload() {
        let raw = r#"{"v":1,"kind":"history_item","id":"x","ts":1}"#;
        let err = decode(raw).expect_err("should fail");
        assert!(err.contains("payload"));
    }

    #[test]
    fn rejects_auth_without_passphrase() {
        let raw = r#"{"v":1,"kind":"auth","id":"x","ts":1,"peer_id":"p"}"#;
        let err = decode(raw).expect_err("should fail");
        assert!(err.contains("passphrase") || err.contains("auth"));
    }
}
