use crate::{
    protocol::{ClientMessage, MAX_WAV_BYTES, ServerMessage},
    state::{AppState, PendingUpload},
};
use axum::{body::Bytes, extract::ws::Message};
use std::{io::Cursor, time::Duration};
use tokio::time::Instant;
use uuid::Uuid;

/// Validate outside the registry lock: even bounded WAV decoding is CPU work.
pub fn validate_wav(data: &[u8], max_duration: Duration) -> Result<(), &'static str> {
    if data.len() < 44 || data.len() > MAX_WAV_BYTES {
        return Err("WAV must contain 44..524288 bytes");
    }
    if &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err("Expected a RIFF/WAVE file");
    }
    let declared = u32::from_le_bytes(data[4..8].try_into().expect("checked header")) as u64 + 8;
    if declared != data.len() as u64 {
        return Err("WAV container length does not match its bytes");
    }
    let mut reader = hound::WavReader::new(Cursor::new(data)).map_err(|_| "Invalid WAV header")?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.sample_rate != 16_000
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err("Expected mono, 16 kHz, signed 16-bit PCM WAV");
    }
    let samples = reader.duration();
    if samples == 0 || samples as f64 / 16_000.0 > max_duration.as_secs_f64() {
        return Err("Audio is empty or exceeds the duration limit");
    }
    for sample in reader.samples::<i16>() {
        sample.map_err(|_| "WAV audio data is truncated or invalid")?;
    }
    Ok(())
}

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
            ClientMessage::BeginUpload {
                recipient_id,
                request_id,
                byte_length,
            } => {
                self.begin_upload(sender, recipient_id, request_id, byte_length);
            }
        }
    }

    fn begin_upload(
        &mut self,
        sender: Uuid,
        recipient: Uuid,
        request_id: Uuid,
        byte_length: usize,
    ) {
        let Some(from) = self.users.get(&sender) else {
            return;
        };
        if *from.cancel.borrow() {
            return;
        }
        if from.pending_upload.is_some() {
            self.error(
                sender,
                "upload_pending",
                "Finish the pending upload before starting another",
            );
            return;
        }
        if sender == recipient {
            self.error(sender, "invalid_recipient", "Choose another user");
            return;
        }
        let Some(to) = self.users.get(&recipient) else {
            self.error(sender, "unknown_recipient", "Recipient is not connected");
            return;
        };
        if *to.cancel.borrow() {
            self.error(sender, "unavailable", "Recipient connection is closing");
            return;
        }
        if !(44..=MAX_WAV_BYTES).contains(&byte_length) {
            self.error(sender, "invalid_size", "WAV must contain 44..524288 bytes");
            return;
        }
        let message_id = Uuid::new_v4();
        if !self.send(
            sender,
            ServerMessage::UploadReady {
                request_id,
                message_id,
            }
            .into_message(),
        ) {
            return;
        }
        self.users
            .get_mut(&sender)
            .expect("connected sender")
            .pending_upload = Some(PendingUpload {
            message_id,
            recipient_id: recipient,
            byte_length,
            started_at: Instant::now(),
        });
        self.broadcast_users();
    }

    /// Called after validation. The next binary message consumes this sender's pending upload.
    pub fn audio(&mut self, sender: Uuid, data: Bytes, validation: Result<(), &'static str>) {
        let Some(user) = self.users.get_mut(&sender) else {
            return;
        };
        if *user.cancel.borrow() {
            return;
        }
        let Some(upload) = user.pending_upload.take() else {
            self.error(
                sender,
                "unexpected_audio",
                "Send BeginUpload and wait for UploadReady first",
            );
            return;
        };
        if upload.started_at.elapsed() >= self.config.upload_timeout {
            self.error(
                sender,
                "upload_timeout",
                "Upload timed out; reconnect before retrying",
            );
            self.cancel_upload_connection(sender);
        } else if data.len() != upload.byte_length {
            self.error(
                sender,
                "size_mismatch",
                "WAV length differs from the announced byte_length",
            );
        } else if let Err(message) = validation {
            self.error(sender, "invalid_wav", message);
        } else {
            let metadata = ServerMessage::AudioMessage {
                message_id: upload.message_id,
                sender_id: sender,
                recipient_id: upload.recipient_id,
                byte_length: data.len(),
            }
            .into_message();
            if self.send_batch(upload.recipient_id, vec![metadata, Message::Binary(data)]) {
                self.send(
                    sender,
                    ServerMessage::MessageAccepted {
                        message_id: upload.message_id,
                    }
                    .into_message(),
                );
            } else {
                self.error(
                    sender,
                    "recipient_unavailable",
                    "Recipient disconnected or cannot accept the message",
                );
            }
        }
        self.broadcast_users();
    }

    fn cancel_upload_connection(&self, sender: Uuid) {
        // With untagged WAV uploads, closing prevents late bytes from being
        // mistaken for a subsequent upload on the same connection.
        if let Some(user) = self.users.get(&sender) {
            let _ = user.cancel.send(true);
        }
    }

    pub fn disconnect(&mut self, id: Uuid) {
        if self.users.remove(&id).is_some() {
            // Other senders retain their pending metadata until their binary
            // message arrives, so it cannot be associated with a new recipient.
            self.broadcast_users();
        }
    }

    pub fn expire(&mut self) {
        let expired: Vec<_> = self
            .users
            .iter()
            .filter_map(|(&id, user)| {
                user.pending_upload
                    .as_ref()
                    .filter(|upload| upload.started_at.elapsed() >= self.config.upload_timeout)
                    .map(|_| id)
            })
            .collect();
        for id in expired {
            self.users
                .get_mut(&id)
                .expect("connected sender")
                .pending_upload = None;
            self.error(
                id,
                "upload_timeout",
                "Upload timed out; reconnect before retrying",
            );
            self.cancel_upload_connection(id);
        }
    }
}
