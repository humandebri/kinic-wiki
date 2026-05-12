// Where: workers/wiki-generator/tests/url-ingest.test.ts
// What: URL ingest request parsing tests.
// Why: Only valid queued request nodes should enter the worker ingest path.
import assert from "node:assert/strict";
import test from "node:test";
import { parseUrlIngestRequest, shouldProcessIngestRequest } from "../src/url-ingest.js";
import type { WikiNode } from "../src/types.js";

const node: WikiNode = {
  path: "/Sources/ingest-requests/1.md",
  kind: "source",
  etag: "etag-1",
  metadataJson: "{}",
  content: [
    "---",
    "kind: kinic.url_ingest_request",
    "schema_version: 1",
    "status: queued",
    'url: "https://example.com/a"',
    'requested_by: "aaaaa-aa"',
    'requested_at: "2026-05-12T00:00:00.000Z"',
    "source_path: null",
    "target_path: null",
    "error: null",
    "---",
    "",
    "# URL Ingest Request"
  ].join("\n")
};

test("valid queued request is parsed", () => {
  const request = parseUrlIngestRequest(node);
  assert.ok(request);
  assert.equal(request.status, "queued");
  assert.equal(request.url, "https://example.com/a");
  assert.equal(shouldProcessIngestRequest(request), true);
});

test("completed request is not processed", () => {
  const request = parseUrlIngestRequest({ ...node, content: node.content.replace("status: queued", "status: completed") });
  assert.ok(request);
  assert.equal(shouldProcessIngestRequest(request), false);
});

test("unrelated source node is ignored", () => {
  assert.equal(parseUrlIngestRequest({ ...node, content: node.content.replace("kinic.url_ingest_request", "kinic.raw_web_source") }), null);
});
