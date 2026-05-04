# Kinic Conversation Capture Usage

Local usage guide for exporting recent ChatGPT conversations into the `local-wiki` canister.

## Prerequisites

- `local-wiki` network is running.
- `wiki` canister is deployed on `local-wiki`.
- Chrome is logged in to ChatGPT.
- This extension is loaded as an unpacked extension.

## Build

```bash
cd extensions/conversation-capture
npm install
npm run build
```

The build creates:

- `dist/content-ui.js`
- `dist/service-worker.js`

## Load in Chrome

1. Open `chrome://extensions`.
2. Enable Developer mode.
3. Select `Load unpacked`.
4. Select `extensions/conversation-capture`.

Do not use `Pack extension` for local testing. `Pack extension` is for producing a `.crx` package and reusing a private key.

## Configure

Find the local `wiki` canister ID:

```bash
icp canister list -e local-wiki
```

Use these extension settings:

- `IC host`: `http://127.0.0.1:8001`
- `Canister ID`: the `wiki` canister ID from `local-wiki`

## Export

1. Open `https://chatgpt.com`.
2. Click the page-level `Kinic Memory` button.
3. Set the recent chat count. The default is `10`.
4. Click `Export`.
5. Watch `Logs` for success or error entries.

The extension fetches ChatGPT conversation data directly from ChatGPT backend API endpoints in the current tab session. It does not navigate the page, open background tabs, use DOM fallback, or use a fetch interceptor.

Raw sources are saved as:

```text
/Sources/raw/chatgpt-<conversationId>/chatgpt-<conversationId>.md
```

## Verify

Check canister status:

```bash
icp canister call wiki status '()' -e local-wiki
```

`source_count` should increase after successful exports.

## Generate Wiki Pages

The extension only writes raw evidence. Generate wiki pages from the CLI:

```bash
cargo run -p vfs-cli -- generate-conversation-wiki --source-path /Sources/raw/chatgpt-<conversationId>/chatgpt-<conversationId>.md
```

## Known Limits

- ChatGPT backend API shape can change.
- Stopping an export can allow up to 2 in-flight conversations to finish saving.
- Public distribution requires write authorization design before release.
