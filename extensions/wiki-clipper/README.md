# Kinic Wiki Clipper

MV3 Chrome extension for creating Kinic Wiki pages from the active tab and saving recent ChatGPT conversations as raw sources.

See [USAGE.md](./USAGE.md) for local canister setup and Chrome loading steps.

ChatGPT raw-source export writes with an anonymous IC actor. URL ingest uses Internet Identity and requires writer access for the selected database.

## Build

```bash
npm install
npm run build
```

Optional build-time database selection hints can be set in `extensions/wiki-clipper/.env`:

```env
KINIC_CAPTURE_DATABASE_ID=db_d36yep4rv43e
```

Load `extensions/wiki-clipper` as an unpacked extension after `dist/service-worker.js`, `dist/content-ui.js`, and `dist/popup.js` exist.
The manifest includes a fixed Chrome extension key. The resulting extension origin is `chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj`. Internet Identity uses `https://xis3j-paaaa-aaaai-axumq-cai.icp0.io` as the derivation origin, and that VFS canister allows both the fixed ID and the old local ID `chrome-extension://hbnicbmdodpmihmcnfgejcdgbfmemoci`.
Open settings from the extension details page `Extension options`.

## Flow

1. Open a ChatGPT conversation tab.
2. Select and save a database from extension settings.
3. Use the page-level `Kinic Memory` button.
4. Enter the number of recent chats to export. The default is `10`.
5. Export to `/Sources/raw/<source_id>/<source_id>.md`.

## Active Tab URL Ingest

Clicking the extension toolbar icon queues the active `http` / `https` tab URL as a wiki ingest request. If settings or Internet Identity login are missing, the extension opens the settings page.

Required settings:

- `Database`: loaded from writable hot databases for the logged-in Internet Identity principal

The active-tab flow writes `/Sources/ingest-requests/<request-id>.md` as a VFS `file`, then triggers `POST /url-ingest` on the generator Worker.

The extension only writes raw evidence. Generate wiki pages later:

```bash
cargo run -p vfs-cli --bin vfs-cli -- generate-conversation-wiki --source-path /Sources/raw/<source_id>/<source_id>.md
```

The CLI creates a conversation wiki scaffold. Re-running it preserves hand-edited scaffold pages unless `--force` is supplied.

## Safety Notes

- Canister ID is fixed to `xis3j-paaaa-aaaai-axumq-cai`.
- IC host is fixed to `https://icp0.io`.
- Generator URL is fixed to `https://wiki-generator.kinic.xyz`.
- Database ID is unset until explicitly saved. `KINIC_CAPTURE_DATABASE_ID` only preselects a matching settings option.
- Mainnet hosts require explicit confirmation before export.
- ChatGPT raw-source writes are anonymous and depend on the target canister accepting `write_node`.
- URL ingest writes use Internet Identity and require writer access for that principal.
- ChatGPT export uses private `/backend-api/*` endpoints. Endpoint shape can change without notice.
- Public release requires owner, allowlist, token, delegation, or equivalent write authorization on the canister.
