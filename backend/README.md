# Walkie-talkie relay backend

Axum/Tokio WebSocket relay for one-to-one, half-duplex audio. No transcription or audio storage is included.

## Run

```sh
cd backend
cargo run
```

The default address is `127.0.0.1:3000`. Set `BIND_ADDR` to override it. `GET /health` returns `ok`; `/ws` accepts WebSocket upgrades. Each connection gets a fresh UUID and a default display name. IDs are connection identities, not authenticated accounts. This MVP is intended for local browser sessions.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## JSON protocol

All controls are WebSocket text messages. UUIDs in JSON use standard UUID strings.

Client messages:

```json
{"type":"set_name","name":"Alice"}
{"type":"start_talk","recipient_id":"<recipient UUID>"}
{"type":"stop_talk","transmission_id":"<transmission UUID>"}
```

Server messages:

- `welcome`: `user_id`. Always the first event.
- `users`: `users`, an array of `{id, name, busy}`. Full snapshots, sent on connection, disconnection, name changes, and reservation changes.
- `talk_started`: `transmission_id`, `sender_id`, `recipient_id`. Sent to both participants; the sender must wait for this grant before sending audio.
- `talk_ended`: `transmission_id`, `reason`. Reasons: `released`, `disconnected`, `audio_timeout`, `duration_limit`.
- `error`: `code`, `message`. Errors include `invalid_message`, `invalid_name`, `invalid_recipient`, `unknown_recipient`, `unavailable`, `busy`, `not_sender`, `invalid_audio`, `unknown_transmission`, and `invalid_sequence`.

Names are trimmed and must contain 1–40 characters with no control characters. Unknown JSON fields are rejected. Each participant can belong to only one active transmission, whether speaking or receiving. Only the sender can explicitly stop it. Duplicate stops for ended transmissions are harmless. Controls and audio from a connection are processed in receive order.

## Audio wire format

Each binary WebSocket message contains one frame:

| Offset | Length | Contents |
| --- | --- | --- |
| 0 | 16 bytes | Raw transmission UUID bytes (standard UUID byte order) |
| 16 | 4 bytes | Sequence number, unsigned little-endian, starting at 0 |
| 20 | 2–640 bytes | Mono 16 kHz signed 16-bit little-endian PCM |

Normal frames contain 320 samples (20 ms); shorter frames are accepted for the final flush. Payloads must have an even length. The server verifies ownership, active transmission, size, and consecutive sequence numbers, then forwards the exact bytes to the recipient only. It does not inspect sample content or enforce real-time pacing. Malformed frames generate an error and do not advance the sequence.

On button release, the browser must finish flushing its capture worklet, send all remaining audio, and then send `stop_talk`. The recipient drains pending playback after `talk_ended`. If release occurs while waiting for a grant, the browser should send stop as soon as the grant arrives. Never reuse an old transmission ID after a timeout or reconnect.

## Execution and limits

- Each connection has one reader and one writer, with a bounded outgoing queue (32 messages).
- Shared state stores channel handles and reservations, not sockets or audio recordings.
- A short mutex section makes reservation changes and enqueue order atomic. No network writes or waiting channel sends occur under this lock.
- A full/closed outgoing queue signals connection cancellation. The connection supervisor stops its reader/writer and removes its reservation. Queued audio may be lost when a connection fails; this is not a graceful release.
- Individual socket writes time out after 2 seconds.
- Transmissions expire after 15 seconds or 3 seconds without accepted audio, checked every 250 ms. These values and queue capacity are in `state::Config`.
- Transport messages and frames are limited to 4096 bytes; audio has the stricter 660-byte limit.
- Client closure, transport errors, cancellation, and write timeouts share the same cleanup path.

## Verification and scope

Integration tests connect actual WebSocket clients to an ephemeral server and check ordered binary delivery, reverse-direction transmission, third-client isolation, malformed data, sender authorization, busy conflicts, concurrent start requests, presence, disconnection, and timeouts. A deterministic queue-congestion test verifies cancellation and continued delivery to unrelated users without relying on OS socket buffer sizes.

These tests validate the relay with synthetic PCM. Actual browser capture, audible playback, jitter buffering, and measured end-to-end audio latency still need client-side verification.

## Scaling trade-offs

Raw PCM costs about 32 KB/s ingress and 32 KB/s egress per active transmission, before headers. WebSockets use reliable ordered transport, so retransmissions can delay later audio. Bounded queues limit application backlog but do not eliminate buffering in TCP or browsers.

Presence broadcasts, the shared state mutex, and a maintenance timer per connection suit a small local demo. Larger deployments should use one maintenance scheduler, partition routing state, cap connections and message rates, and avoid sending full presence snapshots on every change. Multiple server instances need explicit cross-instance routing or conversation affinity. Authentication, origin validation, and TLS are necessary additions for an internet deployment.

Future transcription should receive audio through a separate bounded path with inference outside the relay's mutex and socket tasks. A slow transcription worker must not delay forwarding.
