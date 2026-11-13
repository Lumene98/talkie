# Talkie

A one-to-one voice-message app: select someone, hold to record, release to send, and receive asynchronously a text transcript of the message.

## Run the demo

Install Docker with Compose, then run from the repository root:

```sh
docker compose up -d --build
docker compose ps
```

Open **http://localhost:5173** in two browsers or separate browser profiles. Set different usernames, select the other user, allow microphone access, and hold the talk button or Space for a short English message. Release to send. The recipient can play the audio while transcription finishes; the transcript then appears for both users.

The first startup downloads the Ollama model, so transcription readiness can take longer than subsequent starts. Internet access is needed for images, dependencies, and the initial model download. Model files remain in the named Docker volume; normal inference runs locally. Use headphones to avoid feedback between the two browsers.

```sh
docker compose logs -f transcription backend
docker compose down
```

Compose exposes the frontend on port 5173, Rust on 8000, and ASR on 8001, all on localhost. Nginx proxies the browser's same-origin `/ws` connection to Rust. The volume is named `asr_ollama_models` to reuse the earlier standalone ASR setup.

## Repository

| Directory | Purpose |
| --- | --- |
| `frontend/` | React, TypeScript, Vite, Tailwind, Zustand, and RecordRTC |
| `backend/` | Rust, Axum, Tokio, WebSocket relay, and ASR worker |
| `asr/` | FastAPI adapter and local Ollama model startup |
| `design/` | HTML design reference and visual inspiration |
| `docs/` | Architecture, decisions |

