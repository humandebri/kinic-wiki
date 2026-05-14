# Wiki Generator Worker

Cloudflare Worker for turning raw sources into review-ready wiki drafts.

## LLM

Generation uses DeepSeek Chat Completions with `deepseek-v4-flash`.
Set `DEEPSEEK_API_KEY` as a Cloudflare secret. `KINIC_WIKI_WORKER_TOKEN` is only the shared secret for the manual `POST /run` endpoint; it is not an LLM API key.

## URL Ingest

The worker scans `/Sources/ingest-requests` for `kinic.url_ingest_request` nodes.
Those request nodes are VFS `file` nodes and act as both the authorization gate and request audit log: they record `requested_by`, `requested_at`, `status`, `source_path`, `target_path`, `finished_at`, and `error`.
The fetched raw web evidence written to `/Sources/raw/<id>/<id>.md` remains a VFS `source` node.
Raw web sources keep URL provenance only. Request/source correspondence is tracked from the request node's `source_path`, not by writing `request_path` back into the raw source.
The browser can also trigger a single request immediately with `POST /url-ingest`:

```json
{ "databaseId": "db_...", "requestPath": "/Sources/ingest-requests/<request-id>.md" }
```

For each queued request it:

1. fetches one `http` or `https` URL with a bounded response size,
2. stores normalized evidence under `/Sources/raw/<id>/<id>.md`,
3. queues the raw source for wiki draft generation,
4. writes the generated draft under `/Wiki/conversations`,
5. updates the request status to `completed` or `failed`.

The worker identity in `KINIC_WIKI_WORKER_IDENTITY_PEM` must have writer access to the target database.
`KINIC_WIKI_DATABASE_IDS` is only for cron polling and can be omitted when requests are triggered by `/url-ingest`.
Use the exact PEM output from `icp identity export <identity-name>`.

## Cloudflare Setup

```bash
pnpm exec wrangler queues create kinic-wiki-generation
pnpm exec wrangler d1 create kinic-wiki-generator
pnpm exec wrangler d1 migrations apply kinic-wiki-generator --remote
pnpm exec wrangler secret put DEEPSEEK_API_KEY
pnpm exec wrangler secret put KINIC_WIKI_WORKER_TOKEN
pnpm exec wrangler secret put KINIC_WIKI_WORKER_IDENTITY_PEM
```

After `d1 create`, copy the returned database id into `wrangler.jsonc`.

PDF, authenticated pages, and multi-URL batching are out of scope for this worker path.
