pub mod asr;
pub mod protocol;
pub mod relay;
pub mod state;
pub mod transcription;
pub mod websocket;

use crate::state::{AppState, Config};
use crate::transcription::{TranscriptionJob, run_worker};
use axum::{Extension, Router, routing::get};
use std::time::Duration;
use tokio::sync::mpsc;

pub fn app(config: Config) -> Router {
    let state = AppState::new(config.clone());
    let (job_tx, job_rx) = mpsc::channel::<TranscriptionJob>(config.queue_capacity);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    tokio::spawn(run_worker(job_rx, client, state.clone()));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(websocket::upgrade))
        .layer(Extension(job_tx))
        .with_state(state)
}
