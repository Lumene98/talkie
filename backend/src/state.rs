use crate::protocol::{ServerMessage, UserInfo};
use axum::extract::ws::Message;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, watch};
use tokio::time::Instant;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Config {
    pub queue_capacity: usize,
    pub max_audio_duration: Duration,
    pub upload_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            queue_capacity: 32,
            max_audio_duration: Duration::from_secs(15),
            upload_timeout: Duration::from_secs(15),
            write_timeout: Duration::from_secs(2),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub struct AppState {
    pub users: HashMap<Uuid, ConnectedUser>,
    pub config: Config,
}

pub struct ConnectedUser {
    pub name: String,
    pub outgoing: mpsc::Sender<Vec<Message>>,
    pub cancel: watch::Sender<bool>,
    pub pending_upload: Option<PendingUpload>,
}

pub struct PendingUpload {
    pub message_id: Uuid,
    pub recipient_id: Uuid,
    pub byte_length: usize,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(config: Config) -> SharedState {
        assert!(
            config.queue_capacity >= 4,
            "queue needs room for lifecycle events"
        );
        Arc::new(Mutex::new(Self {
            users: HashMap::new(),
            config,
        }))
    }

    pub fn send(&self, id: Uuid, message: Message) -> bool {
        self.send_batch(id, vec![message])
    }

    // A single queue item keeps metadata and its WAV adjacent on the wire.
    pub fn send_batch(&self, id: Uuid, messages: Vec<Message>) -> bool {
        let Some(user) = self.users.get(&id) else {
            return false;
        };
        if *user.cancel.borrow() {
            return false;
        }
        if user.outgoing.try_send(messages).is_err() {
            let _ = user.cancel.send(true);
            return false;
        }
        true
    }

    pub fn error(&self, id: Uuid, code: &'static str, message: &'static str) {
        self.send(id, ServerMessage::Error { code, message }.into_message());
    }

    pub fn broadcast_users(&self) {
        let mut users: Vec<_> = self
            .users
            .iter()
            .map(|(&id, user)| UserInfo {
                id,
                name: user.name.clone(),
                busy: user.pending_upload.is_some(),
            })
            .collect();
        users.sort_by_key(|user| user.id);
        let message = ServerMessage::Users { users }.into_message();
        for &id in self.users.keys() {
            self.send(id, message.clone());
        }
    }
}
