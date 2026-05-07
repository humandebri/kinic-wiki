# Kinic Conversation Capture

MV3 Chrome extension for saving recent ChatGPT conversations as Kinic Wiki raw sources.

See [USAGE.md](./USAGE.md) for local canister setup and Chrome loading steps.

This is a local-only development extension. It writes with an anonymous IC actor and is not safe for public distribution without canister-side write authorization.

## Build

```bash
npm install
npm run build
```

Load `extensions/conversation-capture` as an unpacked extension after `dist/service-worker.js` and `dist/content-ui.js` exist.

## Flow

1. Open a ChatGPT conversation tab.
2. Set canister ID and local IC host in the extension popup or injected panel.
3. Use the page-level `Kinic Memory` button.
4. Enter the number of recent chats to export. The default is `10`.
5. Export to `/Sources/raw/<source_id>/<source_id>.md`.

The extension only writes raw evidence. Generate wiki pages later:

```bash
cargo run -p vfs-cli -- generate-conversation-wiki --source-path /Sources/raw/<source_id>/<source_id>.md
```

The CLI creates a conversation wiki scaffold. Re-running it preserves hand-edited scaffold pages unless `--force` is supplied.

## Safety Notes

- Default host is `http://127.0.0.1:8001`.
- Mainnet hosts require explicit confirmation before export.
- Writes are anonymous and depend on the target canister accepting `write_node`.
- ChatGPT export uses private `/backend-api/*` endpoints. Endpoint shape can change without notice.
- Public release requires owner, allowlist, token, delegation, or equivalent write authorization on the canister.
