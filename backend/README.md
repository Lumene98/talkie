# Voice-message relay backend

Axum/Tokio backend for delivering complete WAV voice messages between connected users. The relay does not yet queue ASR jobs or send transcripts. The separate ASR client and opt-in sample test are available.

## Run

From `backend/`, run `cargo run`. The default address is `127.0.0.1:3000`; override it with `BIND_ADDR`. `GET /health` returns `ok`; `/ws` accepts WebSocket connections.

Each connection receives a fresh UUID. These are connection identities, not authenticated accounts. Presence is in memory; offline delivery and stored chat history are not implemented.

## Protocol

All controls are JSON text messages. Audio is a complete binary WAV, including its original header. The old PCM frame headers and start/stop-talk controls are no longer supported.

1. Server sends `welcome { user_id }`, then presence snapshots via `users { users: [{id, name, busy}] }`.
2. Client may send `set_name { name }` to change its display name.
3. Sender sends `begin_upload { recipient_id, request_id, byte_length }`.
4. Server responds with `upload_ready { request_id, message_id }` after validating the recipient and announced size.
5. Sender sends exactly one binary WebSocket message containing the complete WAV. This is associated with that connection's pending upload; no additional binary header is used.
6. Recipient receives `audio_message { message_id, sender_id, recipient_id, byte_length }` immediately followed by the binary WAV. They are one outgoing queue item so another sender's message cannot interleave between them.
7. Sender receives `message_accepted { message_id }` after validation and successful recipient queueing. This does not acknowledge receipt, playback, or transcription.

`request_id` correlates the upload grant with its request; it is not an idempotency key. Every accepted upload request gets a new server-generated message ID. Retrying an uncertain delivery may duplicate it.

Only one upload may be pending per sender. Recipients are not reserved: they can receive uploads from multiple people and send while receiving. `busy` indicates that the user has a pending outgoing upload. Presence updates on connection, disconnection, name changes, and upload-state changes.

## Validation and failures

- WAVs must be mono, 16 kHz, signed 16-bit integer PCM, nonempty, and at most 15 seconds long.
- Files are limited to 512 KiB. The exact binary length must match `byte_length`.
- Validation checks the RIFF size, format, duration, and readable audio samples outside the shared-state lock. Accepted WAV bytes are delivered unchanged.
- JSON controls are limited to 4096 bytes. Unknown fields and invalid controls are rejected.
- Names must contain 1–40 non-control characters after trimming.
- Server errors contain `code` and `message`. Codes include `invalid_message`, `invalid_name`, `upload_pending`, `invalid_recipient`, `unknown_recipient`, `unavailable`, `invalid_size`, `unexpected_audio`, `size_mismatch`, `invalid_wav`, `recipient_unavailable`, and `upload_timeout`.
- A second upload request is rejected while the first is pending. The first remains active.
- A mismatched or invalid binary upload consumes the pending request; the sender can begin a new upload afterward.
- A recipient who disconnects before delivery causes rejection, not acknowledgment. Pending metadata is retained until the sender submits its binary message or times out, avoiding accidental reassignment of late bytes.
- Uploads time out after 15 seconds, checked every 250 ms and when a payload arrives. The sender connection closes to prevent a late, untagged WAV from being mistaken for a subsequent upload. The timeout error is best effort; the client must also handle connection closure.
- Transport messages larger than 512 KiB close the connection.

Limits for upload timeout, audio duration, queue capacity, and socket write timeout are in `state::Config`. File and control byte limits are in `protocol.rs`.

## Execution and scaling

Each connection has a reader, writer, and bounded outgoing queue (32 batches by default). Network writes never occur under the shared-state mutex. A full or closed recipient queue rejects delivery and cancels that recipient connection. Writes time out after 2 seconds. Connection cleanup removes its pending upload and broadcasts updated presence.

Batching preserves audio-message association but does not make network delivery transactional. A disconnect can interrupt a payload even after server acceptance. There are no recipient receipts or persistence yet.

The local MVP uses full presence snapshots, a shared state mutex, and a maintenance timer per connection. At scale, use a central expiry scheduler, connection/rate limits, byte-based queue budgets, partitioned routing, and durable audio storage. A queue can retain up to roughly 16 MiB of WAV payloads per connection at current limits, plus overhead and transport buffers. Multi-instance delivery needs explicit routing or affinity. Public deployment also needs authentication, origin validation, and TLS.

Next, add a bounded ASR job queue after successful audio delivery. Inference must run independently of connection handling, with results correlated by message ID.

## Tests

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

WebSocket tests deliver all three files from `asr/samples` byte-for-byte. They cover private routing, acknowledgments, concurrent uploads, metadata/payload adjacency, invalid uploads, retries, disconnection, timeout closure, WAV validation, and queue congestion.

The ASR test sends complete sample WAVs to the existing service and checks for nonempty Ollama `message.content`. It prints transcripts for manual review; it does not measure accuracy against reference text. With the ASR service and model ready on port 8001:

```sh
cargo test --test asr_samples -- --ignored --nocapture
```

That live test is ignored by default and does not run as part of relay verification.
