use crate::protocol::{ServerMessage, UserInfo};
use axum::extract::ws::Message;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, watch};
use tokio::time::Instant;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Config {
    pub queue_capacity: usize,
    pub max_talk_duration: Duration,
    pub audio_idle_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            queue_capacity: 32,
            max_talk_duration: Duration::from_secs(15),
            audio_idle_timeout: Duration::from_secs(3),
            write_timeout: Duration::from_secs(2),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub struct AppState {
    pub users: HashMap<Uuid, ConnectedUser>,
    pub transmissions: HashMap<Uuid, Transmission>,
    pub config: Config,
}

pub struct ConnectedUser {
    pub name: String,
    pub outgoing: mpsc::Sender<Message>,
    pub cancel: watch::Sender<bool>,
    pub active_transmission: Option<Uuid>,
}

pub struct Transmission {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub next_sequence: u32,
    pub started_at: Instant,
    pub last_audio_at: Instant,
}

impl AppState {
    pub fn new(config: Config) -> SharedState {
        assert!(
            config.queue_capacity >= 4,
            "queue needs room for lifecycle events"
        );
        Arc::new(Mutex::new(Self {
            users: HashMap::new(),
            transmissions: HashMap::new(),
            config,
        }))
    }

    pub fn send(&self, id: Uuid, message: Message) {
        if let Some(user) = self.users.get(&id)
            && user.outgoing.try_send(message).is_err()
        {
            let _ = user.cancel.send(true);
        }
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
                busy: user.active_transmission.is_some(),
            })
            .collect();
        users.sort_by_key(|user| user.id);
        let message = ServerMessage::Users { users }.into_message();
        for &id in self.users.keys() {
            self.send(id, message.clone());
        }
    }
}
