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
async fn start(sender: &mut Socket, recipient: Uuid) -> Uuid {
    control(
        sender,
        json!({"type":"start_talk", "recipient_id":recipient}),
    )
    .await;
    Uuid::parse_str(
        event(sender, "talk_started").await["transmission_id"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
fn audio(id: Uuid, sequence: u32, bytes: usize) -> Vec<u8> {
    let mut frame = id.as_bytes().to_vec();
    frame.extend(sequence.to_le_bytes());
    frame.extend((0..bytes).map(|i| (i % 255) as u8));
    frame
}
async fn binary(socket: &mut Socket) -> Vec<u8> {
    loop {
        if let Message::Binary(data) = next(socket).await {
            return data.to_vec();
        }
    }
}

#[tokio::test]
async fn routes_ordered_audio_and_release_then_reverse_direction() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    let (mut c, _) = server.connect().await;
    assert_ne!(aid, bid);
    let id = start(&mut a, bid).await;
    event(&mut b, "talk_started").await;
    for sequence in 0..3 {
        let frame = audio(id, sequence, if sequence == 2 { 2 } else { 640 });
        a.send(Message::Binary(frame.clone().into())).await.unwrap();
        assert_eq!(binary(&mut b).await, frame);
    }
    control(&mut a, json!({"type":"stop_talk", "transmission_id":id})).await;
    assert_eq!(event(&mut b, "talk_ended").await["reason"], "released");
    event(&mut a, "talk_ended").await;
    // Stopping twice is harmless and doesn't prevent a reverse transmission.
    control(&mut a, json!({"type":"stop_talk", "transmission_id":id})).await;
    let reverse = start(&mut b, aid).await;
    event(&mut a, "talk_started").await;
    let frame = audio(reverse, 0, 640);
    b.send(Message::Binary(frame.clone().into())).await.unwrap();
    assert_eq!(binary(&mut a).await, frame);
    // The third client may receive presence, but never private audio or lifecycle events.
    assert!(
        tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match next(&mut c).await {
                    Message::Binary(_) => panic!("audio leaked to third client"),
                    Message::Text(text) => assert_eq!(
                        serde_json::from_str::<Value>(&text).unwrap()["type"],
                        "users"
                    ),
                    _ => {}
                }
            }
        })
        .await
        .is_err()
    );
}

#[tokio::test]
async fn validates_requests_frames_and_sender_ownership() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    control(&mut a, json!({"type":"start_talk", "recipient_id":aid})).await;
    assert_eq!(event(&mut a, "error").await["code"], "invalid_recipient");
    control(
        &mut a,
        json!({"type":"start_talk", "recipient_id":Uuid::new_v4()}),
    )
    .await;
    assert_eq!(event(&mut a, "error").await["code"], "unknown_recipient");
    a.send(Message::Text("not json".into())).await.unwrap();
    assert_eq!(event(&mut a, "error").await["code"], "invalid_message");
    let id = start(&mut a, bid).await;
    event(&mut b, "talk_started").await;
    control(&mut b, json!({"type":"start_talk", "recipient_id":aid})).await;
    assert_eq!(event(&mut b, "error").await["code"], "busy");
    control(&mut b, json!({"type":"stop_talk", "transmission_id":id})).await;
    assert_eq!(event(&mut b, "error").await["code"], "not_sender");
    b.send(Message::Binary(audio(id, 0, 640).into()))
        .await
        .unwrap();
    assert_eq!(event(&mut b, "error").await["code"], "not_sender");
    for frame in [vec![], vec![0; 19], audio(id, 0, 1), audio(id, 0, 642)] {
        a.send(Message::Binary(frame.into())).await.unwrap();
        assert_eq!(event(&mut a, "error").await["code"], "invalid_audio");
    }
    a.send(Message::Binary(audio(id, 1, 640).into()))
        .await
        .unwrap();
    assert_eq!(event(&mut a, "error").await["code"], "invalid_sequence");
    let valid = audio(id, 0, 640);
    a.send(Message::Binary(valid.clone().into())).await.unwrap();
    assert_eq!(binary(&mut b).await, valid);
}

#[tokio::test]
async fn updates_presence_and_disconnect_frees_the_survivor() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    control(&mut a, json!({"type":"set_name", "name":" Alice "})).await;
    loop {
        let users = event(&mut b, "users").await;
        if users["users"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["name"] == "Alice")
        {
            break;
        }
    }
    start(&mut a, bid).await;
    event(&mut b, "talk_started").await;
    a.close(None).await.unwrap();
    assert_eq!(event(&mut b, "talk_ended").await["reason"], "disconnected");
    let users = event(&mut b, "users").await;
    assert!(
        !users["users"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["id"] == aid.to_string())
    );
    let (mut c, cid) = server.connect().await;
    start(&mut b, cid).await;
    event(&mut c, "talk_started").await;
    // Receiver disconnect also releases the sender.
    c.close(None).await.unwrap();
    assert_eq!(event(&mut b, "talk_ended").await["reason"], "disconnected");
}

#[tokio::test]
async fn simultaneous_start_requests_grant_exactly_one_transmission() {
    let server = Server::start(Config::default()).await;
    let (mut a, aid) = server.connect().await;
    let (mut b, bid) = server.connect().await;
    tokio::join!(
        control(&mut a, json!({"type":"start_talk","recipient_id":bid})),
        control(&mut b, json!({"type":"start_talk","recipient_id":aid}))
    );
    let first = event(&mut a, "talk_started").await;
    let second = event(&mut b, "talk_started").await;
    assert_eq!(first["transmission_id"], second["transmission_id"]);
    let loser = if first["sender_id"] == aid.to_string() {
        &mut b
    } else {
        &mut a
    };
    assert_eq!(event(loser, "error").await["code"], "busy");
}

#[tokio::test]
async fn inactivity_and_duration_limits_release_users() {
    for (idle, max, reason) in [
        (
            Duration::from_millis(100),
            Duration::from_secs(10),
            "audio_timeout",
        ),
        (
            Duration::from_secs(10),
            Duration::from_millis(100),
            "duration_limit",
        ),
    ] {
        let server = Server::start(Config {
            audio_idle_timeout: idle,
            max_talk_duration: max,
            ..Config::default()
        })
        .await;
        let (mut a, _) = server.connect().await;
        let (mut b, bid) = server.connect().await;
        start(&mut a, bid).await;
        assert_eq!(event(&mut b, "talk_ended").await["reason"], reason);
        start(&mut a, bid).await;
    }
}

#[tokio::test]
async fn congested_connection_is_cancelled_without_blocking_other_users() {
    use axum::extract::ws::Message as ServerWireMessage;
    use backend::{
        protocol::ClientMessage,
        state::{AppState, ConnectedUser},
    };
    use tokio::sync::{mpsc, watch};
    let shared = AppState::new(Config::default());
    let mut state = shared.lock().await;
    let sender = Uuid::new_v4();
    let slow = Uuid::new_v4();
    let healthy = Uuid::new_v4();
    let mut queues = Vec::new();
    let mut cancellations = Vec::new();
    for id in [sender, slow, healthy] {
        let (outgoing, incoming) = mpsc::channel(8);
        let (cancel, cancelled) = watch::channel(false);
        state.users.insert(
            id,
            ConnectedUser {
                name: "test".into(),
                outgoing,
                cancel,
                active_transmission: None,
            },
        );
        queues.push(incoming);
        cancellations.push(cancelled);
    }
    state.control(sender, ClientMessage::StartTalk { recipient_id: slow });
    let talk = state.users[&sender].active_transmission.unwrap();
    for seq in 0..12 {
        state.audio(sender, audio(talk, seq, 640).into());
    }
    assert!(*cancellations[1].borrow());
    assert!(!*cancellations[0].borrow());
    assert!(!*cancellations[2].borrow());
    // Connection supervision calls disconnect after observing cancellation.
    state.disconnect(slow);
    assert!(state.users[&sender].active_transmission.is_none());
    assert!(!state.transmissions.contains_key(&talk));
    state.send(healthy, ServerWireMessage::Text("still responsive".into()));
    let mut found = false;
    while let Ok(message) = queues[2].try_recv() {
        if let ServerWireMessage::Text(text) = message {
            found |= text == "still responsive";
        }
    }
    assert!(found);
}
