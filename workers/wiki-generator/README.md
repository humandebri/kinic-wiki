# Wiki Generator Worker

Cloudflare Worker for turning raw sources into review-ready wiki drafts.

## URL Ingest

The worker scans `/Sources/ingest-requests` for `kinic.url_ingest_request` nodes.
For each queued request it:

1. fetches one `http` or `https` URL with a bounded response size,
2. stores normalized evidence under `/Sources/raw/<id>/<id>.md`,
3. queues the raw source for wiki draft generation,
4. writes the generated draft under `/Wiki/conversations`,
5. updates the request status to `completed` or `failed`.

The worker identity in `KINIC_WIKI_WORKER_IDENTITY_JSON` must have writer access to every database listed in `KINIC_WIKI_DATABASE_IDS`.

PDF, authenticated pages, and multi-URL batching are out of scope for this worker path.
