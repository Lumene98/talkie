use crate::{
    protocol::{ClientMessage, ServerMessage},
    state::{ConnectedUser, SharedState},
};
use axum::{
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
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    upgrade
        .max_message_size(4096)
        .max_frame_size(4096)
        .on_upgrade(move |socket| connection(socket, state))
}

async fn connection(socket: WebSocket, state: SharedState) {
    let id = Uuid::new_v4();
    let config = state.lock().await.config.clone();
    let (outgoing, mut messages) = mpsc::channel(config.queue_capacity);
    let (cancel, mut cancelled) = watch::channel(false);
    outgoing
        .try_send(ServerMessage::Welcome { user_id: id }.into_message())
        .expect("empty queue");
    {
        let mut app = state.lock().await;
        app.users.insert(
            id,
            ConnectedUser {
                name: format!("User {}", &id.to_string()[..8]),
                outgoing,
                cancel,
                active_transmission: None,
            },
        );
        app.broadcast_users();
    }
    let (mut sink, mut stream) = socket.split();
    let reader = async {
        while let Some(Ok(message)) = stream.next().await {
            match message {
                Message::Text(text) => {
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
                Message::Binary(data) => state.lock().await.audio(id, data),
                Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }
    };
    let writer = async {
        while let Some(message) = messages.recv().await {
            if !matches!(
                tokio::time::timeout(config.write_timeout, sink.send(message)).await,
                Ok(Ok(()))
            ) {
                break;
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
