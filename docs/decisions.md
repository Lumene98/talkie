## Main choices

| Choice | Reason | Trade-off |
| --- | --- | --- |
| Complete WAV on release | Simple recording, validation, playback, and inference input | Recipient cannot hear speech while it is being recorded |
| WebSockets for controls and audio | One bidirectional connection for presence and message delivery | Reliable ordered transport can delay later messages behind large payloads |
| Rust relay with a separate ASR worker | Inference does not block audio delivery | One slow transcription delays subsequent jobs |
| Local Qwen3-ASR via Ollama | Adds useful searchable/readable text without hosted inference credentials | Model startup and inference depend on local compute |
| English language prefix | Reduces language ambiguity in short English utterances | Multilingual transcription is outside the current demo |
| In-memory connections and chat state | Keeps the MVP small | Restart, reload, and reconnect lose state |
| Tailwind and native controls | Recreates the reference with direct markup and browser controls | Some rendering, particularly audio controls, varies by browser |

The visual concept borrows the orbital composition and warm accents of the [design reference](../design/README.md). Circles are people with flat avatars, not literal planets. Actions are direct: select a user, record, play, and read. The talk control appears only inside a selected chat.

## AI contribution

Transcription makes a voice message usable when listening is inconvenient and lets users scan previous exchanges. The original audio remains available because a transcript can mishear names, numbers, accents, or short repeated words.

The implementation uses a pretrained speech model; it does not train or fine-tune one. Samples establish that the integration returns text, not measured transcription accuracy. A next evaluation would compare a labeled set of short recordings across accents, background noise, and repeated words, measuring word error rate and end-to-end latency.

Libraries and AI-assisted development were used for implementation, troubleshooting, refactoring, and documentation. 

## Concepts beyond the MVP

- **Live streaming:** introduce a stream ID, sequence numbers, start/end controls, and bounded audio chunks. Capture with AudioWorklet, define a codec/sample format, and add receiver buffering. Benchmark WebSocket streaming against WebRTC for interactive audio.
- **Partial captions:** use incremental inference or bounded overlapping windows. Send revision IDs and partial/final flags so the UI replaces provisional text correctly.
- **One speaker at a time:** maintain a server-owned conversation lease, granted on talk request and released on stop, timeout, or disconnect. The current upload flag is not that lease.
- **Stable identities and offline presence:** separate users from connections, add authenticated resume tokens, and define duplicate-session behavior. Retain profiles after disconnect; derive online status from active connections.
- **History and offline delivery:** store metadata in a database and audio in object storage, with retention limits, authorization, recipient receipts, and idempotency keys.

These are proposals, not features already implemented.

