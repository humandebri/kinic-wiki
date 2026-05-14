// Where: workers/wiki-generator/tests/url-ingest.test.ts
// What: URL ingest request parsing tests.
// Why: Only valid queued request nodes should enter the worker ingest path.
import assert from "node:assert/strict";
import test from "node:test";
import { parseUrlIngestRequest, parseUrlIngestTriggerInput, processUrlIngestRequest, shouldProcessIngestRequest } from "../src/url-ingest.js";
import type { UrlIngestRequest, WikiNode } from "../src/types.js";
import { testEnv, TestQueue, TestVfsClient, withFetchedPage, workerConfig } from "./url-ingest-fixtures.js";

const node: WikiNode = {
  path: "/Sources/ingest-requests/1.md",
  kind: "file",
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
    "finished_at: null",
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
  assert.equal(request.finishedAt, null);
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

test("source-kind request node is ignored", () => {
  assert.equal(parseUrlIngestRequest({ ...node, kind: "source" }), null);
});

test("url ingest trigger input carries database and request path", () => {
  assert.deepEqual(parseUrlIngestTriggerInput({ databaseId: "db_1", requestPath: "/Sources/ingest-requests/1.md" }), {
    databaseId: "db_1",
    requestPath: "/Sources/ingest-requests/1.md"
  });
  assert.equal(parseUrlIngestTriggerInput({ databaseId: "db_1" }), "requestPath is required");
});

test("queued URL ingest uses source write ack without reading source after write", async () => {
  const vfs = new TestVfsClient();
  const queue = new TestQueue();

  await withFetchedPage(async () => {
    await processUrlIngestRequest(testEnv(queue), vfs, workerConfig(), "db_1", queuedRequest());
  });

  assert.equal(vfs.sourceReadsBeforeWrite, 1);
  assert.equal(vfs.sourceReadsAfterWrite, 0);
  assert.equal(queue.messages.length, 1);
  assert.equal(queue.messages[0]?.sourceEtag, "etag-source-write");
  assert.equal(vfs.lastRequest?.status, "generating");
  assert.equal(vfs.lastRequest?.sourcePath, queue.messages[0]?.sourcePath);
  assert.equal(vfs.lastRequest?.finishedAt, null);
  assert.ok(vfs.lastSourceWrite);
  assert.doesNotMatch(vfs.lastSourceWrite.content, /request_path/);
  assert.doesNotMatch(vfs.lastSourceWrite.metadataJson, /request_path/);
});

test("queued URL ingest fails when write_node returns a non-source ack", async () => {
  const vfs = new TestVfsClient();
  vfs.sourceAckKind = "file";
  const queue = new TestQueue();

  await withFetchedPage(async () => {
    await processUrlIngestRequest(testEnv(queue), vfs, workerConfig(), "db_1", queuedRequest());
  });

  assert.equal(queue.messages.length, 0);
  assert.equal(vfs.lastRequest?.status, "failed");
  assert.match(vfs.lastRequest?.finishedAt ?? "", /^\d{4}-\d{2}-\d{2}T/);
  assert.match(vfs.lastRequest?.error ?? "", /non-source kind/);
});

test("completed URL ingest request records finished_at", async () => {
  const vfs = new TestVfsClient();
  vfs.existingSource = {
    path: "/Sources/raw/existing/existing.md",
    kind: "source",
    content: "raw",
    etag: "etag-existing-source",
    metadataJson: "{}"
  };
  const queue = new TestQueue();

  await processUrlIngestRequest(
    testEnv(queue),
    vfs,
    workerConfig(),
    "db_1",
    queuedRequest({ status: "source_written", sourcePath: "/Sources/raw/existing/existing.md" })
  );

  assert.equal(vfs.lastRequest?.status, "completed");
  assert.equal(vfs.lastRequest?.targetPath, "/Wiki/conversations/a.md");
  assert.match(vfs.lastRequest?.finishedAt ?? "", /^\d{4}-\d{2}-\d{2}T/);
});

test("completed URL ingest request preserves existing finished_at", async () => {
  const vfs = new TestVfsClient();
  vfs.existingSource = {
    path: "/Sources/raw/existing/existing.md",
    kind: "source",
    content: "raw",
    etag: "etag-existing-source",
    metadataJson: "{}"
  };
  const queue = new TestQueue();

  await processUrlIngestRequest(
    testEnv(queue),
    vfs,
    workerConfig(),
    "db_1",
    queuedRequest({
      status: "source_written",
      sourcePath: "/Sources/raw/existing/existing.md",
      finishedAt: "2026-05-13T00:00:00.000Z"
    })
  );

  assert.equal(vfs.lastRequest?.status, "completed");
  assert.equal(vfs.lastRequest?.finishedAt, "2026-05-13T00:00:00.000Z");
});

test("source_written URL ingest still reads source to recover etag", async () => {
  const vfs = new TestVfsClient();
  vfs.existingSource = {
    path: "/Sources/raw/retry/retry.md",
    kind: "source",
    content: "raw",
    etag: "etag-existing-source",
    metadataJson: "{}"
  };
  const queue = new TestQueue();

  await processUrlIngestRequest(
    testEnv(queue),
    vfs,
    workerConfig(),
    "db_1",
    queuedRequest({ status: "source_written", sourcePath: "/Sources/raw/retry/retry.md" })
  );

  assert.equal(vfs.sourceReadsBeforeWrite, 1);
  assert.equal(vfs.sourceWrites, 0);
  assert.equal(queue.messages[0]?.sourceEtag, "etag-existing-source");
  assert.equal(queue.messages[0]?.sourcePath, "/Sources/raw/retry/retry.md");
});

function queuedRequest(overrides: Partial<UrlIngestRequest> = {}): UrlIngestRequest {
  return {
    path: "/Sources/ingest-requests/1.md",
    etag: "etag-request",
    status: "queued",
    url: "https://example.com/a",
    requestedBy: "aaaaa-aa",
    requestedAt: "2026-05-12T00:00:00.000Z",
    sourcePath: null,
    targetPath: null,
    finishedAt: null,
    error: null,
    ...overrides
  };
}
