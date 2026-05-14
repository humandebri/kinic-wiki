// Where: workers/wiki-generator/tests/index.test.ts
// What: Entrypoint authorization and handler-shape tests.
// Why: Public triggers must stay bearer-protected, and cron polling is disabled.
import assert from "node:assert/strict";
import test from "node:test";
import worker from "../src/index.js";
import { testEnv, TestQueue } from "./url-ingest-fixtures.js";

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

test("worker does not expose scheduled cron handler", () => {
  assert.equal("scheduled" in worker, false);
});

function urlIngestRequest(): Request {
  return new Request("https://wiki-generator.kinic.xyz/url-ingest", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      databaseId: "db_1",
      requestPath: "/Sources/ingest-requests/1.md"
    })
  });
}

function fetchWorker(request: Request, env: ReturnType<typeof testEnv>): Promise<Response> {
  if (!worker.fetch) throw new Error("fetch handler is required");
  return Promise.resolve(worker.fetch(request, env, ctx()));
}

function ctx(): ExecutionContext {
  return {
    waitUntil(_promise: Promise<unknown>) {}
  };
}
