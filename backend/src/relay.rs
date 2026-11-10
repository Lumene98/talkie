use crate::{
    protocol::{ClientMessage, ServerMessage, parse_audio},
    state::{AppState, Transmission},
};
use axum::extract::ws::Message;
use tokio::time::Instant;
use uuid::Uuid;

impl AppState {
    pub fn control(&mut self, sender: Uuid, message: ClientMessage) {
        match message {
            ClientMessage::SetName { name } => {
                let name = name.trim();
                if name.is_empty()
                    || name.chars().count() > 40
                    || name.chars().any(char::is_control)
                {
                    self.error(
                        sender,
                        "invalid_name",
                        "Name must contain 1..40 printable characters",
                    );
                    return;
                }
                if let Some(user) = self.users.get_mut(&sender) {
                    user.name = name.to_owned();
                }
                self.broadcast_users();
            }
            ClientMessage::StartTalk { recipient_id } => self.start_talk(sender, recipient_id),
            ClientMessage::StopTalk { transmission_id } => {
                if let Some(talk) = self.transmissions.get(&transmission_id) {
                    if talk.sender_id != sender {
                        self.error(
                            sender,
                            "not_sender",
                            "Only the sender can stop this transmission",
                        );
                        return;
                    }
                    self.end_talk(transmission_id, "released");
                }
                // Repeated stops are harmless after release, timeout, or disconnect.
            }
        }
    }

    fn start_talk(&mut self, sender: Uuid, recipient: Uuid) {
        if sender == recipient {
            self.error(sender, "invalid_recipient", "Choose another user");
            return;
        }
        let (Some(from), Some(to)) = (self.users.get(&sender), self.users.get(&recipient)) else {
            self.error(sender, "unknown_recipient", "Recipient is not connected");
            return;
        };
        if *from.cancel.borrow() || *to.cancel.borrow() {
            self.error(sender, "unavailable", "Connection is closing");
            return;
        }
        if from.active_transmission.is_some() || to.active_transmission.is_some() {
            self.error(sender, "busy", "One of the participants is already busy");
            return;
        }
        let id = Uuid::new_v4();
        let now = Instant::now();
        self.transmissions.insert(
            id,
            Transmission {
                sender_id: sender,
                recipient_id: recipient,
                next_sequence: 0,
                started_at: now,
                last_audio_at: now,
            },
        );
        for user_id in [sender, recipient] {
            self.users
                .get_mut(&user_id)
                .expect("validated participant")
                .active_transmission = Some(id);
            self.send(
                user_id,
                ServerMessage::TalkStarted {
                    transmission_id: id,
                    sender_id: sender,
                    recipient_id: recipient,
                }
                .into_message(),
            );
        }
        self.broadcast_users();
    }

    pub fn audio(&mut self, sender: Uuid, data: axum::body::Bytes) {
        let (id, sequence) = match parse_audio(&data) {
            Ok(frame) => frame,
            Err(message) => {
                self.error(sender, "invalid_audio", message);
                return;
            }
        };
        let Some(talk) = self.transmissions.get_mut(&id) else {
            self.error(sender, "unknown_transmission", "Transmission is not active");
            return;
        };
        if talk.sender_id != sender {
            self.error(
                sender,
                "not_sender",
                "Connection does not own this transmission",
            );
            return;
        }
        if sequence != talk.next_sequence || sequence == u32::MAX {
            self.error(
                sender,
                "invalid_sequence",
                "Expected the next sequence number, beginning at zero",
            );
            return;
        }
        talk.next_sequence += 1;
        talk.last_audio_at = Instant::now();
        let recipient = talk.recipient_id;
        self.send(recipient, Message::Binary(data));
    }

    pub fn end_talk(&mut self, id: Uuid, reason: &'static str) {
        let Some(talk) = self.transmissions.remove(&id) else {
            return;
        };
        for participant in [talk.sender_id, talk.recipient_id] {
            if let Some(user) = self.users.get_mut(&participant)
                && user.active_transmission == Some(id)
            {
                user.active_transmission = None;
            }
            self.send(
                participant,
                ServerMessage::TalkEnded {
                    transmission_id: id,
                    reason,
                }
                .into_message(),
            );
        }
        self.broadcast_users();
    }

    pub fn disconnect(&mut self, id: Uuid) {
        if let Some(user) = self.users.remove(&id) {
            if let Some(talk_id) = user.active_transmission {
                self.end_talk(talk_id, "disconnected");
            } else {
                self.broadcast_users();
            }
        }
    }

    pub fn expire(&mut self) {
        let now = Instant::now();
        let expired: Vec<_> = self
            .transmissions
            .iter()
            .filter_map(|(&id, talk)| {
                if now.duration_since(talk.started_at) >= self.config.max_talk_duration {
                    Some((id, "duration_limit"))
                } else if now.duration_since(talk.last_audio_at) >= self.config.audio_idle_timeout {
                    Some((id, "audio_timeout"))
                } else {
                    None
                }
            })
            .collect();
        for (id, reason) in expired {
            self.end_talk(id, reason);
        }
    }
}
