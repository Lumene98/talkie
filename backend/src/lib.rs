pub mod asr;
pub mod protocol;
pub mod relay;
pub mod state;
mod utils;
pub mod websocket;

use axum::{Router, routing::get};
use state::{AppState, Config};

pub fn app(config: Config) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(websocket::upgrade))
        .with_state(AppState::new(config))
}
