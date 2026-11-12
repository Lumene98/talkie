use crate::protocol::ServerMessage;
use crate::state::SharedState;
use axum::body::Bytes;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct TranscriptionJob {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub wav: Bytes,
}

pub async fn run_worker(
    mut jobs: mpsc::Receiver<TranscriptionJob>,
    client: reqwest::Client,
    state: SharedState,
    asr_url: String,
) {
    while let Some(job) = jobs.recv().await {
        let result = crate::asr::transcribe(&client, &asr_url, job.wav).await;

        let event = match result {
            Ok(transcript) => ServerMessage::TranscriptReady {
                message_id: job.message_id,
                transcript_body: transcript,
            }
            .into_message(),
            Err(error) => {
                eprintln!("Transcription failed for {}: {error}", job.message_id);
                ServerMessage::TranscriptFailed {
                    message_id: job.message_id,
                    reason: "Transcription is unavailable".to_owned(),
                }
                .into_message()
            }
        };

        let state = state.lock().await;

        state.send(job.sender_id, event.clone());
        state.send(job.recipient_id, event);
    }
}
