# Frontend

React, TypeScript, Vite, Tailwind CSS, Zustand, and RecordRTC. Native buttons, a modal dialog, and audio controls implement the [HTML design reference](../design/index.html); shadcn and Radix are not used.

## Local development

Use Node 24 and pnpm 10.30.3, matching the Docker build. From this directory:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Vite proxies `/ws` to `http://127.0.0.1:3000`, the default address of `cargo run` in `backend/`. To use the Docker backend instead:

```sh
TALKIE_BACKEND=http://127.0.0.1:8000 pnpm dev
```

If Docker's frontend already occupies port 5173, Vite may choose another port; open the address it prints. `pnpm preview` is a static build preview and does not configure the development WebSocket proxy. For a working production demo, use the root Compose setup.

## Structure

- `src/components/`: people, chats, chat, profile, and shared avatar components.
- `src/store/`: session, chat messages, and upload state.
- `src/net/`: WebSocket connection and event dispatch.
- `src/audio/`: microphone recording and playback helpers.
- `src/protocol/`: message types, validation, and limits.
- `src/hooks/`: pointer and keyboard hold-to-record handling.
- `src/lib/`: display formatting and shared utilities.

## Checks and formatting

```sh
pnpm lint
pnpm lint:fix
pnpm format
pnpm format:check
pnpm build
```
