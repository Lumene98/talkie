use axum::extract::ws::Message as AxumMessage;
use backend::{
    app,
    protocol::ClientMessage,
    state::{AppState, Config, ConnectedUser},
    transcription::TranscriptionJob,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
    _asr: Option<MockAsr>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start(mut config: Config) -> Self {
        let mock = MockAsr::start(vec![]).await;
        config.asr_url = mock.url.clone();
        let mut server = Self::with_asr(config).await;
        server._asr = Some(mock);
        server
    }
    async fn with_asr(config: Config) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app(config)).await.unwrap();
        });
        Self {
            url,
            task,
            _asr: None,
        }
    }
    async fn connect(&self) -> (Socket, Uuid) {
        let (mut socket, _) = connect_async(&self.url).await.unwrap();
        let welcome = event(&mut socket, "welcome").await;
        let id = Uuid::parse_str(welcome["user_id"].as_str().unwrap()).unwrap();
        (socket, id)
    }
}
async fn next(socket: &mut Socket) -> Message {
    tokio::time::timeout(Duration::from_secs(3), socket.next())
        .await
        .expect("message timeout")
        .expect("connection closed")
        .expect("websocket error")
}
async fn event(socket: &mut Socket, kind: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Message::Text(text) = next(socket).await {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] == kind {
                    return value;
                }
            }
        }
    })
    .await
    .expect("event timeout")
}
async fn control(socket: &mut Socket, value: Value) {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

const HELLO: &[u8] = include_bytes!("../../asr/samples/01_hello.wav");
const NUMBERS: &[u8] = include_bytes!("../../asr/samples/02_numbers.wav");
const LONG: &[u8] = include_bytes!("../../asr/samples/03_long_message.wav");

async fn begin(sender: &mut Socket, recipient: Uuid, length: usize) -> Uuid {
    let request_id = Uuid::new_v4();
    control(sender,json!({"type":"begin_upload","recipient_id":recipient,"request_id":request_id,"byte_length":length})).await;
    let ready = event(sender, "upload_ready").await;
    assert_eq!(ready["request_id"], request_id.to_string());
    Uuid::parse_str(ready["message_id"].as_str().unwrap()).unwrap()
}

async fn received(socket: &mut Socket, message_id: Uuid, sender_id: Uuid, expected: &[u8]) {
    let metadata = event(socket, "audio_message").await;
    assert_eq!(metadata["message_id"], message_id.to_string());
    assert_eq!(metadata["sender_id"], sender_id.to_string());
    assert_eq!(metadata["byte_length"], expected.len());
    // Nothing, including presence, may separate metadata from its payload.
    match next(socket).await {
        Message::Binary(bytes) => assert_eq!(bytes.as_ref(), expected),
        other => panic!("Expected WAV immediately after metadata, got {other:?}"),
    }
}

#[tokio::test]
async fn delivers_all_sample_wavs_privately_and_acknowledges_sender() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    let (mut observer, _) = server.connect().await;
    for sample in [HELLO, NUMBERS, LONG] {
        let id = begin(&mut a, bid, sample.len()).await;
        a.send(Message::Binary(sample.to_vec().into()))
            .await
            .unwrap();
        received(&mut b, id, aid, sample).await;
        assert_eq!(
            event(&mut a, "message_accepted").await["message_id"],
            id.to_string()
        );
    }
    let id = begin(&mut b, aid, HELLO.len()).await;
    b.send(Message::Binary(HELLO.to_vec().into()))
        .await
        .unwrap();
    received(&mut a, id, bid, HELLO).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match next(&mut observer).await {
                    Message::Text(text) => assert_eq!(
                        serde_json::from_str::<Value>(&text).unwrap()["type"],
                        "users"
                    ),
                    other => panic!("Private message leaked: {other:?}"),
                }
            }
        })
        .await
        .is_err()
    );
}

#[tokio::test]
async fn rejects_invalid_uploads_and_allows_retry_after_consumed_payload() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    a.send(Message::Binary(HELLO.to_vec().into()))
        .await
        .unwrap();
    assert_eq!(event(&mut a, "error").await["code"], "unexpected_audio");
    for (recipient, size, code) in [
        (aid, HELLO.len(), "invalid_recipient"),
        (Uuid::new_v4(), HELLO.len(), "unknown_recipient"),
        (bid, 0, "invalid_size"),
        (bid, 524289, "invalid_size"),
    ] {
        control(&mut a,json!({"type":"begin_upload","recipient_id":recipient,"request_id":Uuid::new_v4(),"byte_length":size})).await;
        assert_eq!(event(&mut a, "error").await["code"], code);
    }
    begin(&mut a, bid, HELLO.len()).await;
    control(&mut a,json!({"type":"begin_upload","recipient_id":bid,"request_id":Uuid::new_v4(),"byte_length":HELLO.len()})).await;
    assert_eq!(event(&mut a, "error").await["code"], "upload_pending");
    a.send(Message::Binary(vec![0; 44].into())).await.unwrap();
    assert_eq!(event(&mut a, "error").await["code"], "size_mismatch");
    begin(&mut a, bid, 44).await;
    a.send(Message::Binary(vec![0; 44].into())).await.unwrap();
    assert_eq!(event(&mut a, "error").await["code"], "invalid_wav");
    let id = begin(&mut a, bid, HELLO.len()).await;
    a.send(Message::Binary(HELLO.to_vec().into()))
        .await
        .unwrap();
    received(&mut b, id, aid, HELLO).await;
}

#[tokio::test]
async fn concurrent_uploads_to_one_recipient_keep_metadata_and_audio_together() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    let (mut c, cid) = server.connect().await;
    let first = begin(&mut a, bid, HELLO.len()).await;
    let second = begin(&mut c, bid, NUMBERS.len()).await;
    let (ra, rc) = tokio::join!(
        a.send(Message::Binary(HELLO.to_vec().into())),
        c.send(Message::Binary(NUMBERS.to_vec().into()))
    );
    ra.unwrap();
    rc.unwrap();
    let mut seen = Vec::new();
    for _ in 0..2 {
        let metadata = event(&mut b, "audio_message").await;
        let id = Uuid::parse_str(metadata["message_id"].as_str().unwrap()).unwrap();
        let (sender, sample) = if id == first {
            (aid, HELLO)
        } else {
            assert_eq!(id, second);
            (cid, NUMBERS)
        };
        assert_eq!(metadata["sender_id"], sender.to_string());
        let Message::Binary(data) = next(&mut b).await else {
            panic!("Interleaved delivery")
        };
        assert_eq!(data.as_ref(), sample);
        seen.push(id);
    }
    assert_ne!(seen[0], seen[1]);
}

#[tokio::test]
async fn recipient_disconnect_rejects_pending_upload_without_retargeting_it() {
    let server = Server::start(Config::default()).await;
    let (mut a, _) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    begin(&mut a, bid, HELLO.len()).await;
    b.close(None).await.unwrap();
    loop {
        let users = event(&mut a, "users").await;
        if !users["users"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["id"] == bid.to_string())
        {
            break;
        }
    }
    a.send(Message::Binary(HELLO.to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        event(&mut a, "error").await["code"],
        "recipient_unavailable"
    );
    let (_c, cid) = server.connect().await;
    begin(&mut a, cid, HELLO.len()).await;
}

#[tokio::test]
async fn timed_out_upload_closes_connection_to_prevent_late_payload_reuse() {
    let server = Server::start(Config {
        upload_timeout: Duration::from_millis(80),
        ..Config::default()
    })
    .await;
    let (mut a, _) = server.connect().await;
    let (_b, bid) = server.connect().await;
    begin(&mut a, bid, HELLO.len()).await;
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match a.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {}
            }
        }
    })
    .await
    .expect("timed out connection should close");
}

#[test]
fn validates_wav_format_duration_and_truncated_audio() {
    use backend::relay::validate_wav;
    for sample in [HELLO, NUMBERS, LONG] {
        validate_wav(sample, Duration::from_secs(15)).unwrap();
    }
    assert!(validate_wav(HELLO, Duration::from_secs(1)).is_err());
    assert!(validate_wav(&HELLO[..HELLO.len() - 2], Duration::from_secs(15)).is_err());
    let mut truncated = HELLO[..HELLO.len() - 2].to_vec();
    let size = (truncated.len() - 8) as u32;
    truncated[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(validate_wav(&truncated, Duration::from_secs(15)).is_err());
    let mut wav = std::io::Cursor::new(Vec::new());
    {
        let writer = hound::WavWriter::new(
            &mut wav,
            hound::WavSpec {
                channels: 2,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        writer.finalize().unwrap();
    }
    assert!(validate_wav(wav.get_ref(), Duration::from_secs(15)).is_err());
}

// State-level helpers let us test queueing without requiring a real ASR service.
fn add_user(
    state: &mut AppState,
    id: Uuid,
    name: &str,
    capacity: usize,
) -> (mpsc::Receiver<Vec<AxumMessage>>, watch::Receiver<bool>) {
    let (outgoing, messages) = mpsc::channel(capacity);
    let (cancel, cancelled) = watch::channel(false);
    state.users.insert(
        id,
        ConnectedUser {
            name: name.to_owned(),
            outgoing,
            cancel,
            pending_upload: None,
        },
    );
    (messages, cancelled)
}

fn begin_state_upload(state: &mut AppState, sender: Uuid, recipient: Uuid) -> Uuid {
    state.control(
        sender,
        ClientMessage::BeginUpload {
            recipient_id: recipient,
            request_id: Uuid::new_v4(),
            byte_length: HELLO.len(),
        },
    );
    state.users[&sender]
        .pending_upload
        .as_ref()
        .expect("upload must be pending")
        .message_id
}

fn drain_text_events(rx: &mut mpsc::Receiver<Vec<AxumMessage>>) -> Vec<Value> {
    let mut events = Vec::new();
    while let Ok(batch) = rx.try_recv() {
        for message in batch {
            if let AxumMessage::Text(text) = message {
                events.push(serde_json::from_str(&text).unwrap());
            }
        }
    }
    events
}

#[tokio::test]
async fn accepted_audio_enqueues_one_transcription_job_with_original_ids_and_wav() {
    let shared = AppState::new(Config::default());
    let mut state = shared.lock().await;
    let sender = Uuid::new_v4();
    let recipient = Uuid::new_v4();
    let (mut sender_rx, _sender_cancelled) = add_user(&mut state, sender, "Sender", 16);
    let (mut recipient_rx, _recipient_cancelled) = add_user(&mut state, recipient, "Recipient", 16);
    let (job_tx, mut job_rx) = mpsc::channel::<TranscriptionJob>(4);
    let message_id = begin_state_upload(&mut state, sender, recipient);

    state.audio(sender, HELLO.to_vec().into(), Ok(()), &job_tx);

    let job = job_rx
        .try_recv()
        .expect("one transcription job should be queued");
    assert_eq!(job.message_id, message_id);
    assert_eq!(job.sender_id, sender);
    assert_eq!(job.recipient_id, recipient);
    // Works whether TranscriptionJob.wav is Bytes or Vec<u8>.
    assert_eq!(job.wav.as_ref(), HELLO);
    assert!(matches!(
        job_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));

    let sender_events = drain_text_events(&mut sender_rx);
    assert!(sender_events.iter().any(|event| {
        event["type"] == "message_accepted" && event["message_id"] == message_id.to_string()
    }));

    // Audio metadata and binary WAV must remain in the same outgoing queue item.
    let mut found_audio = false;
    while let Ok(batch) = recipient_rx.try_recv() {
        if let [AxumMessage::Text(metadata), AxumMessage::Binary(wav)] = batch.as_slice() {
            let metadata: Value = serde_json::from_str(metadata).unwrap();
            if metadata["type"] == "audio_message" {
                assert_eq!(metadata["message_id"], message_id.to_string());
                assert_eq!(metadata["sender_id"], sender.to_string());
                assert_eq!(wav.as_ref(), HELLO);
                found_audio = true;
            }
        }
    }
    assert!(
        found_audio,
        "recipient must receive the WAV with its metadata"
    );
}

#[tokio::test]
async fn invalid_audio_does_not_queue_transcription() {
    let shared = AppState::new(Config::default());
    let mut state = shared.lock().await;
    let sender = Uuid::new_v4();
    let recipient = Uuid::new_v4();
    let (mut sender_rx, _sender_cancelled) = add_user(&mut state, sender, "Sender", 16);
    let (mut recipient_rx, _recipient_cancelled) = add_user(&mut state, recipient, "Recipient", 16);
    let (job_tx, mut job_rx) = mpsc::channel::<TranscriptionJob>(4);
    begin_state_upload(&mut state, sender, recipient);

    state.audio(
        sender,
        HELLO.to_vec().into(),
        Err("Invalid WAV header"),
        &job_tx,
    );

    let sender_events = drain_text_events(&mut sender_rx);
    assert!(
        sender_events
            .iter()
            .any(|event| event["code"] == "invalid_wav")
    );
    assert!(
        !sender_events
            .iter()
            .any(|event| event["type"] == "message_accepted")
    );
    let recipient_events = drain_text_events(&mut recipient_rx);
    assert!(
        !recipient_events
            .iter()
            .any(|event| event["type"] == "audio_message")
    );
    assert!(matches!(
        job_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn transcription_queue_full_or_closed_notifies_both_participants_without_losing_audio() {
    for close_receiver in [false, true] {
        let shared = AppState::new(Config::default());
        let mut state = shared.lock().await;
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let (mut sender_rx, _sender_cancelled) = add_user(&mut state, sender, "Sender", 16);
        let (mut recipient_rx, _recipient_cancelled) =
            add_user(&mut state, recipient, "Recipient", 16);
        let (job_tx, mut job_rx) = mpsc::channel::<TranscriptionJob>(1);
        let message_id = begin_state_upload(&mut state, sender, recipient);

        if close_receiver {
            job_rx.close();
        } else {
            // Fill the queue before submitting the audio being tested.
            job_tx
                .try_send(TranscriptionJob {
                    message_id: Uuid::new_v4(),
                    sender_id: sender,
                    recipient_id: recipient,
                    wav: HELLO.to_vec().into(),
                })
                .unwrap();
        }

        state.audio(sender, HELLO.to_vec().into(), Ok(()), &job_tx);

        let sender_events = drain_text_events(&mut sender_rx);
        assert!(sender_events.iter().any(|event| {
            event["type"] == "message_accepted" && event["message_id"] == message_id.to_string()
        }));
        assert!(sender_events.iter().any(|event| {
            event["type"] == "transcript_failed" && event["message_id"] == message_id.to_string()
        }));

        let mut recipient_failed = false;
        let mut recipient_got_audio = false;
        while let Ok(batch) = recipient_rx.try_recv() {
            match batch.as_slice() {
                [AxumMessage::Text(metadata), AxumMessage::Binary(wav)] => {
                    let metadata: Value = serde_json::from_str(metadata).unwrap();
                    if metadata["type"] == "audio_message" {
                        assert_eq!(metadata["message_id"], message_id.to_string());
                        assert_eq!(wav.as_ref(), HELLO);
                        recipient_got_audio = true;
                    }
                }
                _ => {
                    for message in batch {
                        if let AxumMessage::Text(text) = message {
                            let event: Value = serde_json::from_str(&text).unwrap();
                            if event["type"] == "transcript_failed" {
                                assert_eq!(event["message_id"], message_id.to_string());
                                recipient_failed = true;
                            }
                        }
                    }
                }
            }
        }
        assert!(recipient_got_audio, "audio should still be delivered");
        assert!(
            recipient_failed,
            "recipient should be notified of ASR unavailability"
        );

        if !close_receiver {
            // The unsuccessful try_send must not replace or append another job.
            assert!(job_rx.try_recv().is_ok());
            assert!(matches!(
                job_rx.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
        }
    }
}

#[tokio::test]
async fn congested_recipient_is_not_acknowledged_or_given_partial_delivery() {
    let shared = AppState::new(Config::default());
    let mut state = shared.lock().await;
    let sender = Uuid::new_v4();
    let recipient = Uuid::new_v4();
    let (mut sender_rx, _sender_cancelled) = add_user(&mut state, sender, "Sender", 16);
    let (mut recipient_rx, recipient_cancelled) = add_user(&mut state, recipient, "Recipient", 1);
    let (job_tx, mut job_rx) = mpsc::channel::<TranscriptionJob>(4);
    begin_state_upload(&mut state, sender, recipient);

    // Presence occupies the recipient's single queue slot.
    state.audio(sender, HELLO.to_vec().into(), Ok(()), &job_tx);
    assert!(*recipient_cancelled.borrow());

    let sender_events = drain_text_events(&mut sender_rx);
    assert!(
        !sender_events
            .iter()
            .any(|event| event["type"] == "message_accepted")
    );
    assert!(
        sender_events
            .iter()
            .any(|event| event["code"] == "recipient_unavailable")
    );

    let batch = recipient_rx.try_recv().unwrap();
    assert_eq!(batch.len(), 1);
    assert!(recipient_rx.try_recv().is_err());
    // No transcription should run for audio that was not delivered.
    assert!(matches!(
        job_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

struct MockAsr {
    url: String,
    requests: mpsc::UnboundedReceiver<(axum::http::HeaderMap, axum::body::Bytes)>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for MockAsr {
    fn drop(&mut self) {
        self.task.abort();
    }
}

type AsrReply = (axum::http::StatusCode, &'static str, Duration);
const TRANSCRIPT: &str = r#"{"message":{"content":"language English<asr_text> Hello there! "}}"#;

impl MockAsr {
    async fn start(replies: Vec<AsrReply>) -> Self {
        use axum::{Router, routing::post};
        use std::{collections::VecDeque, sync::Arc};
        let replies = Arc::new(tokio::sync::Mutex::new(VecDeque::from(replies)));
        let (tx, requests) = mpsc::unbounded_channel();
        let router =
            Router::new().route(
                "/transcribe",
                post(
                    move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                        let replies = replies.clone();
                        let tx = tx.clone();
                        async move {
                            let _ = tx.send((headers, body));
                            let (status, body, delay) = replies.lock().await.pop_front().unwrap_or(
                                (axum::http::StatusCode::OK, TRANSCRIPT, Duration::ZERO),
                            );
                            tokio::time::sleep(delay).await;
                            (status, body)
                        }
                    },
                ),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/transcribe", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            url,
            requests,
            task,
        }
    }

    async fn assert_wav_request(&mut self, wav: &[u8]) {
        let (headers, body) = tokio::time::timeout(Duration::from_secs(3), self.requests.recv())
            .await
            .unwrap()
            .unwrap();
        let content_type = headers["content-type"].to_str().unwrap();
        assert!(content_type.starts_with("multipart/form-data; boundary="));
        let boundary = content_type.split("boundary=").nth(1).unwrap();
        let prefix = format!("--{boundary}\r\n");
        assert!(body.starts_with(prefix.as_bytes()));
        let header_end = body.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let part_headers = std::str::from_utf8(&body[..header_end]).unwrap();
        assert!(part_headers.contains("name=\"file\""));
        assert!(part_headers.contains("filename=\"audio.wav\""));
        assert!(part_headers.contains("audio/wav"));
        let payload_start = header_end + 4;
        assert_eq!(&body[payload_start..payload_start + wav.len()], wav);
        assert_eq!(
            &body[payload_start + wav.len()..],
            format!("\r\n--{boundary}--\r\n").as_bytes()
        );
    }
}

async fn upload_and_receive(
    sender: &mut Socket,
    sender_id: Uuid,
    recipient: &mut Socket,
    recipient_id: Uuid,
) -> Uuid {
    let id = begin(sender, recipient_id, HELLO.len()).await;
    sender
        .send(Message::Binary(HELLO.to_vec().into()))
        .await
        .unwrap();
    received(recipient, id, sender_id, HELLO).await;
    assert_eq!(
        event(sender, "message_accepted").await["message_id"],
        id.to_string()
    );
    id
}

async fn assert_transcript(socket: &mut Socket, id: Uuid, kind: &str) {
    let result = event(socket, kind).await;
    assert_eq!(result["message_id"], id.to_string());
    if kind == "transcript_ready" {
        assert_eq!(result["transcript_body"], "Hello there!");
    } else {
        assert_eq!(result["reason"], "Transcription is unavailable");
    }
}

#[tokio::test]
async fn wav_reaches_asr_and_transcript_reaches_both_clients() {
    let mut asr = MockAsr::start(vec![]).await;
    let server = Server::with_asr(Config {
        asr_url: asr.url.clone(),
        ..Config::default()
    })
    .await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    let id = upload_and_receive(&mut a, aid, &mut b, bid).await;
    asr.assert_wav_request(HELLO).await;
    assert_transcript(&mut a, id, "transcript_ready").await;
    assert_transcript(&mut b, id, "transcript_ready").await;
}

#[tokio::test]
async fn asr_failures_notify_both_clients_and_worker_processes_next_job() {
    use axum::http::StatusCode;
    for reply in [
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            Duration::ZERO,
        ),
        (StatusCode::OK, "not json", Duration::ZERO),
        (StatusCode::OK, r#"{"message":{}}"#, Duration::ZERO),
        (
            StatusCode::OK,
            r#"{"message":{"content":"<asr_text> "}}"#,
            Duration::ZERO,
        ),
        (StatusCode::OK, TRANSCRIPT, Duration::from_secs(1)),
    ] {
        let mut asr = MockAsr::start(vec![reply]).await;
        let server = Server::with_asr(Config {
            asr_url: asr.url.clone(),
            asr_timeout: Duration::from_millis(250),
            ..Config::default()
        })
        .await;
        let (mut a, aid) = server.connect().await;
        let (mut b, bid) = server.connect().await;
        let failed_id = upload_and_receive(&mut a, aid, &mut b, bid).await;
        asr.assert_wav_request(HELLO).await;
        assert_transcript(&mut a, failed_id, "transcript_failed").await;
        assert_transcript(&mut b, failed_id, "transcript_failed").await;

        let next_id = upload_and_receive(&mut a, aid, &mut b, bid).await;
        assert_ne!(next_id, failed_id);
        asr.assert_wav_request(HELLO).await;
        assert_transcript(&mut a, next_id, "transcript_ready").await;
        assert_transcript(&mut b, next_id, "transcript_ready").await;
    }
}
