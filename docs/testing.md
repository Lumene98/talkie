# Testing

Run commands from the repository root unless stated otherwise.

## Automated checks

```sh
cargo test --manifest-path backend/Cargo.toml
cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings
pnpm --dir frontend lint
pnpm --dir frontend format:check
pnpm --dir frontend build
docker compose config --quiet
```

Install frontend dependencies first with `pnpm --dir frontend install --frozen-lockfile`.

The Rust integration suite covers sample WAV delivery, private routing, concurrent senders, metadata/audio adjacency, invalid payloads, retries, disconnection, upload timeout, congestion, and ASR queue handling. A local HTTP stub verifies multipart WAV submission, transcript correlation for both participants, and recovery after ASR errors and timeouts.

The ordinary suite does not require Ollama. To test the real service after startup:

```sh
cargo test --manifest-path backend/Cargo.toml --test asr_samples -- --ignored --nocapture
```

That opt-in test submits the three WAV samples under `asr/samples/` and checks nonempty transcripts. It does not measure accuracy. Frontend lint and build checks do not prove browser behavior; there is no automated browser test suite or load benchmark.

## Manual two-browser checklist

1. Start Compose and open `http://localhost:5173` in two browsers. Choose distinct usernames and verify that each sees the other.
2. Select the peer, grant microphone permission, record a short English sentence, and release. Verify the recipient receives playable audio and both messages receive a transcript.
3. Send a response in the opposite direction. Check sender names, timestamps, and transcript association.
4. Test pointer release outside the button and the Space shortcut. Confirm typing in the username dialog does not record.
5. Deny microphone permission and verify that the UI reports failure and allows retry after permissions are corrected.
6. Record to the duration limit. Verify automatic stop and delivery of a valid WAV.
7. Close one browser. Verify server presence removes it; a partner with local chat history may remain visible as offline. Recording to that partner must be unavailable.
8. Reload or reconnect. Expect a fresh identity and cleared local chats, not session restoration.
9. Temporarily stop ASR with `docker compose stop transcription`. Send a message: audio should still arrive and the transcript should fail. Restart with `docker compose start transcription`, wait for readiness, and send a new message. Failed transcripts are not retried automatically.
10. Check a narrow viewport, keyboard focus, the native username dialog, and audio playback in both browsers.

Use headphones. Browser microphone access requires localhost or a secure origin. The Compose ports are bound to loopback, so the documented demo runs on one machine.

## Troubleshooting

- **Reconnecting:** check the browser console and WebSocket request, then `docker compose logs --tail=100 frontend backend`. The frontend must reach `/ws` through Nginx or the Vite proxy.
- **Audio arrives but no transcript:** inspect `docker compose logs --tail=100 transcription backend`. Confirm the model finished downloading and the ASR queue is not waiting on earlier requests.
- **Wrong words:** replay the original audio. English is selected, but recognition is not guaranteed; very short utterances provide little context.
- **Container build certificate errors:** configure the trusted CA for the network/environment. Do not disable TLS verification.
