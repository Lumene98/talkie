use backend::{app, state::Config};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start(config: Config) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app(config)).await.unwrap();
        });
        Self { url, task }
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

#[tokio::test]
async fn congested_recipient_is_not_acknowledged_or_given_partial_delivery() {
    use backend::{
        protocol::ClientMessage,
        state::{AppState, ConnectedUser},
    };
    use tokio::sync::{mpsc, watch};
    let shared = AppState::new(Config::default());
    let mut state = shared.lock().await;
    let sender = Uuid::new_v4();
    let recipient = Uuid::new_v4();
    let (sender_tx, mut sender_rx) = mpsc::channel(16);
    let (recipient_tx, mut recipient_rx) = mpsc::channel(1);
    let (sender_cancel, _) = watch::channel(false);
    let (recipient_cancel, recipient_cancelled) = watch::channel(false);
    state.users.insert(
        sender,
        ConnectedUser {
            name: "Sender".into(),
            outgoing: sender_tx,
            cancel: sender_cancel,
            pending_upload: None,
        },
    );
    state.users.insert(
        recipient,
        ConnectedUser {
            name: "Recipient".into(),
            outgoing: recipient_tx,
            cancel: recipient_cancel,
            pending_upload: None,
        },
    );
    state.control(
        sender,
        ClientMessage::BeginUpload {
            recipient_id: recipient,
            request_id: Uuid::new_v4(),
            byte_length: HELLO.len(),
        },
    );
    // Presence occupies the recipient's single queue slot.
    state.audio(sender, HELLO.to_vec().into(), Ok(()));
    assert!(*recipient_cancelled.borrow());
    let mut rejected = false;
    while let Ok(batch) = sender_rx.try_recv() {
        for msg in batch {
            if let axum::extract::ws::Message::Text(text) = msg {
                let value: Value = serde_json::from_str(&text).unwrap();
                assert_ne!(value["type"], "message_accepted");
                rejected |= value["code"] == "recipient_unavailable";
            }
        }
    }
    assert!(rejected);
    let batch = recipient_rx.try_recv().unwrap();
    assert_eq!(batch.len(), 1);
    assert!(recipient_rx.try_recv().is_err());
}
