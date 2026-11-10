use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AUDIO_HEADER_BYTES: usize = 20;
pub const MAX_PCM_BYTES: usize = 640;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    SetName { name: String },
    StartTalk { recipient_id: Uuid },
    StopTalk { transmission_id: Uuid },
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub busy: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        user_id: Uuid,
    },
    Users {
        users: Vec<UserInfo>,
    },
    TalkStarted {
        transmission_id: Uuid,
        sender_id: Uuid,
        recipient_id: Uuid,
    },
    TalkEnded {
        transmission_id: Uuid,
        reason: &'static str,
    },
    Error {
        code: &'static str,
        message: &'static str,
    },
}

impl ServerMessage {
    pub fn into_message(self) -> Message {
        Message::Text(
            serde_json::to_string(&self)
                .expect("serialize server event")
                .into(),
        )
    }
}

pub fn parse_audio(data: &[u8]) -> Result<(Uuid, u32), &'static str> {
    let payload_len = data.len().saturating_sub(AUDIO_HEADER_BYTES);
    if payload_len == 0 || payload_len > MAX_PCM_BYTES || !payload_len.is_multiple_of(2) {
        return Err("Expected a 20-byte header and 2..640 even PCM bytes");
    }
    let id = Uuid::from_slice(&data[..16]).map_err(|_| "Invalid transmission ID")?;
    let sequence = u32::from_le_bytes(data[16..20].try_into().expect("validated header"));
    Ok((id, sequence))
}
