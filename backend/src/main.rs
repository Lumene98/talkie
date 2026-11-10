use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(|| async { "Helthy!" }))
        .route("/ws", get(ws_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(result) = socket.recv().await {
        match result {
            Ok(message @ (Message::Text(_) | Message::Binary(_))) => {
                if socket.send(message).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => {}
        }
    }
}
