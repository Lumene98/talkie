use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_WAV_BYTES: usize = 512 * 1024;
pub const MAX_CONTROL_BYTES: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    SetName {
        name: String,
    },
    BeginUpload {
        recipient_id: Uuid,
        request_id: Uuid,
        byte_length: usize,
    },
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
    AudioMessage {
        message_id: Uuid,
        sender_id: Uuid,
        recipient_id: Uuid,
        byte_length: usize,
    },
    UploadReady {
        request_id: Uuid,
        message_id: Uuid,
    },
    MessageAccepted {
        message_id: Uuid,
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
