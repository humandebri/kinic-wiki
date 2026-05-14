// Where: workers/wiki-generator/tests/index.test.ts
// What: Entrypoint authorization and handler-shape tests.
// Why: Public triggers must stay bearer-protected, and cron polling is disabled.
import assert from "node:assert/strict";
import test from "node:test";
import worker from "../src/index.js";
import { testEnv, TestQueue } from "./url-ingest-fixtures.js";

Object.defineProperty(crypto.subtle, "timingSafeEqual", {
  configurable: true,
  value(left: Uint8Array, right: Uint8Array): boolean {
    if (left.length !== right.length) return false;
    return left.every((value, index) => value === right[index]);
  }
});

test("url ingest trigger requires worker token config", async () => {
  const response = await fetchWorker(urlIngestRequest(), { ...testEnv(new TestQueue()), KINIC_WIKI_WORKER_TOKEN: "" });

  assert.equal(response.status, 503);
  assert.match(await response.text(), /KINIC_WIKI_WORKER_TOKEN is required/);
});

test("url ingest trigger rejects missing bearer token", async () => {
  const response = await fetchWorker(urlIngestRequest(), testEnv(new TestQueue()));

  assert.equal(response.status, 401);
  assert.match(await response.text(), /unauthorized/);
});

test("url ingest trigger rejects invalid request path before background work", async () => {
  const context = recordingCtx();
  const response = await fetchWorker(authorizedUrlIngestRequest({ requestPath: "/Sources/raw/1.md" }), testEnv(new TestQueue()), context);

  assert.equal(response.status, 400);
  assert.match(await response.text(), /non-canonical ingest request path/);
  assert.equal(context.waitUntilCount, 0);
});

test("url ingest trigger rejects missing canister config before background work", async () => {
  const context = recordingCtx();
  const response = await fetchWorker(authorizedUrlIngestRequest(), { ...testEnv(new TestQueue()), KINIC_WIKI_CANISTER_ID: "" }, context);

  assert.equal(response.status, 500);
  assert.match(await response.text(), /KINIC_WIKI_CANISTER_ID is required/);
  assert.equal(context.waitUntilCount, 0);
});

test("url ingest trigger rejects canister mismatches before background work", async () => {
  const context = recordingCtx();
  const response = await fetchWorker(authorizedUrlIngestRequest({ canisterId: "aaaaa-aa" }), testEnv(new TestQueue()), context);

  assert.equal(response.status, 400);
  assert.match(await response.text(), /canisterId does not match worker canister config/);
  assert.equal(context.waitUntilCount, 0);
});

test("worker does not expose scheduled cron handler", () => {
  assert.equal("scheduled" in worker, false);
});

function authorizedUrlIngestRequest(body: Record<string, string> = {}): Request {
  return urlIngestRequest({ authorization: "Bearer worker-token" }, body);
}

function urlIngestRequest(headers: Record<string, string> = {}, body: Record<string, string> = {}): Request {
  return new Request("https://wiki-generator.kinic.xyz/url-ingest", {
    method: "POST",
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify({
      canisterId: "xis3j-paaaa-aaaai-axumq-cai",
      databaseId: "db_1",
      requestPath: "/Sources/ingest-requests/1.md",
      ...body
    })
  });
}

function fetchWorker(request: Request, env: ReturnType<typeof testEnv>, executionContext: ExecutionContext = ctx()): Promise<Response> {
  if (!worker.fetch) throw new Error("fetch handler is required");
  return Promise.resolve(worker.fetch(request, env, executionContext));
}

function ctx(): ExecutionContext {
  return {
    waitUntil(_promise: Promise<unknown>) {}
  };
}

function recordingCtx(): ExecutionContext & { waitUntilCount: number } {
  return {
    waitUntilCount: 0,
    waitUntil(_promise: Promise<unknown>) {
      this.waitUntilCount += 1;
    }
  };
}
