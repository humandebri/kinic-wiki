# Kinic Wiki Clipper Usage

Usage guide for exporting recent ChatGPT conversations and active-tab URLs into the mainnet Kinic Wiki canister.

ChatGPT raw-source export uses an anonymous write-capable actor. URL ingest uses Internet Identity and requires writer access for the selected database.

## Prerequisites

- Chrome is logged in to ChatGPT.
- This extension is loaded as an unpacked extension.

## Build

```bash
cd extensions/wiki-clipper
npm install
cat > .env <<'EOF'
KINIC_CAPTURE_DATABASE_ID=<database-id>
EOF
npm run build
```

`KINIC_CAPTURE_DATABASE_ID` only preselects a matching database in settings. It is not used as an automatic write target.

The build creates:

- `dist/content-ui.js`
- `dist/offscreen.js`
- `dist/popup.js`
- `dist/service-worker.js`

## Load in Chrome

1. Open `chrome://extensions`.
2. Enable Developer mode.
3. Select `Load unpacked`.
4. Select `extensions/wiki-clipper`.

Do not use `Pack extension` for local testing. `Pack extension` is for producing a `.crx` package and reusing a private key.
The extension has a fixed manifest key, so local unpacked installs use `chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj`. Internet Identity derives principals from `https://xis3j-paaaa-aaaai-axumq-cai.icp0.io`; that canister also accepts the old local ID `chrome-extension://hbnicbmdodpmihmcnfgejcdgbfmemoci`.

## Configure

Open settings from `chrome://extensions` → Kinic Wiki Clipper → `Extension options`.

Use these extension settings:

- `Database`: select a writable hot database for the logged-in Internet Identity principal

The extension fixes canister ID to `xis3j-paaaa-aaaai-axumq-cai`, IC host to `https://icp0.io`, and generator URL to `https://wiki-generator.kinic.xyz`. The database must already exist. Mainnet writes require explicit confirmation before ChatGPT raw export.

Login with Internet Identity from the extension settings page, select a writable database, and save it before clicking the toolbar icon. The logged-in principal must have writer access to the selected database.

## Export

1. Open `https://chatgpt.com`.
2. Click the page-level `Kinic Memory` button.
3. Set the recent chat count. The default is `10`.
4. Click `Export`.
5. Watch `Logs` for success or error entries.

The extension fetches ChatGPT conversation data directly from ChatGPT backend API endpoints in the current tab session. It does not navigate the page, open background tabs, use DOM fallback, or use a fetch interceptor.

Those `/backend-api/*` endpoints are private ChatGPT internals. If ChatGPT changes the response shape, export can fail or omit messages.

Raw sources are saved as:

```text
/Sources/raw/chatgpt-<conversationId>/chatgpt-<conversationId>.md
```

## URL Ingest

1. Open any public `http` / `https` page.
2. Click the extension toolbar icon.
3. The extension writes `/Sources/ingest-requests/<request-id>.md`.
4. The generator Worker fetches the URL and creates `/Sources/raw/...` plus `/Wiki/conversations/...`.

Non-web pages such as `chrome://extensions` are rejected.

## Verify

Confirm that `/Sources/raw/...` or `/Sources/ingest-requests/...` is created in the selected database after successful exports.

## Generate Wiki Pages

The extension only writes raw evidence. Generate wiki pages from the CLI:

```bash
cargo run -p vfs-cli --bin vfs-cli -- generate-conversation-wiki --source-path /Sources/raw/chatgpt-<conversationId>/chatgpt-<conversationId>.md
```

This command creates a wiki scaffold. Re-running it preserves existing `summary.md`, `facts.md`, `events.md`, `plans.md`, `preferences.md`, and `open_questions.md`. Use `--force` only when those pages should be regenerated.

## Known Limits

- ChatGPT backend API shape can change.
- Stopping an export can allow up to 2 in-flight conversations to finish saving.
- ChatGPT raw-source writes are anonymous. Public distribution requires write authorization design before release.
