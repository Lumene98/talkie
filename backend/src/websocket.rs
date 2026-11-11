use crate::{
    protocol::{ClientMessage, MAX_CONTROL_BYTES, MAX_WAV_BYTES, ServerMessage},
    relay::validate_wav,
    state::{ConnectedUser, SharedState},
    transcription::TranscriptionJob,
};
use axum::{
    Extension,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

pub async fn upgrade(
    State(state): State<SharedState>,
    Extension(job_tx): Extension<mpsc::Sender<TranscriptionJob>>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    upgrade
        .max_message_size(MAX_WAV_BYTES)
        .max_frame_size(MAX_WAV_BYTES)
        .on_upgrade(move |socket| connection(socket, state, job_tx))
}

async fn connection(socket: WebSocket, state: SharedState, job_tx: mpsc::Sender<TranscriptionJob>) {
    let id = Uuid::new_v4();
    let config = state.lock().await.config.clone();

    let (outgoing, mut messages) = mpsc::channel(config.queue_capacity);
    let (cancel, mut cancelled) = watch::channel(false);
    outgoing
        .try_send(vec![ServerMessage::Welcome { user_id: id }.into_message()])
        .expect("empty queue");
    {
        let mut app = state.lock().await;
        app.users.insert(
            id,
            ConnectedUser {
                name: format!("User {}", &id.to_string()[..8]),
                outgoing,
                cancel,
                pending_upload: None,
            },
        );
        app.broadcast_users();
    }
    let (mut sink, mut stream) = socket.split();
    let reader = async {
        while let Some(Ok(message)) = stream.next().await {
            match message {
                Message::Text(text) => {
                    if text.len() > MAX_CONTROL_BYTES {
                        state.lock().await.error(
                            id,
                            "invalid_message",
                            "Control messages must not exceed 4096 bytes",
                        );
                        continue;
                    }
                    let parsed = serde_json::from_str::<ClientMessage>(&text);
                    let mut app = state.lock().await;
                    match parsed {
                        Ok(control) => app.control(id, control),
                        Err(_) => app.error(
                            id,
                            "invalid_message",
                            "Expected a valid JSON control message",
                        ),
                    }
                }
                Message::Binary(data) => {
                    let validation = validate_wav(&data, config.max_audio_duration);
                    state.lock().await.audio(id, data, validation, &job_tx);
                }
                Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }
    };
    let writer = async {
        while let Some(batch) = messages.recv().await {
            for message in batch {
                if !matches!(
                    tokio::time::timeout(config.write_timeout, sink.send(message)).await,
                    Ok(Ok(()))
                ) {
                    return;
                }
            }
        }
    };

    let maintenance = async {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            tick.tick().await;
            state.lock().await.expire();
        }
    };
    // Dropping the other futures closes both socket halves when either side ends.
    tokio::select! {
        _ = reader => {},
        _ = writer => {},
        _ = cancelled.wait_for(|value| *value) => {},
        _ = maintenance => {},
    }
    state.lock().await.disconnect(id);
}
