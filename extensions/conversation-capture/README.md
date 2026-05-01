# Kinic Conversation Capture

MV3 Chrome extension for saving recent ChatGPT conversations as Kinic Wiki raw sources.

## Build

```bash
npm install
npm run build
```

Load `extensions/conversation-capture` as an unpacked extension after `dist/service-worker.js` and `dist/content-ui.js` exist.

## Flow

1. Open a ChatGPT conversation tab.
2. Set canister ID and IC host in the extension popup or injected panel.
3. Use the page-level `Kinic Memory` button.
4. Enter the number of recent chats to export. The default is `10`.
5. Export to `/Sources/raw/<source_id>/<source_id>.md`.

The extension only writes raw evidence. Generate wiki pages later:

```bash
cargo run -p vfs-cli -- generate-conversation-wiki --source-path /Sources/raw/<source_id>/<source_id>.md
```
