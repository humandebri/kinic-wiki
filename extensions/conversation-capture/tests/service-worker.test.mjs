// Where: extensions/conversation-capture/tests/service-worker.test.mjs
// What: Unit tests for DB-scoped canister writes from the service worker.
// Why: The capture extension must call the current canister API shape.
import assert from "node:assert/strict";
import test from "node:test";
import { saveSource } from "../src/service-worker.js";

test("saveSource passes database id to read_node and write_node", async () => {
  const calls = [];
  const result = await saveSource(
    capture(),
    { canisterId: "aaaaa-aa", databaseId: "team-db", host: "http://127.0.0.1:8001" },
    {
      storage: memoryStorage(),
      async createVfsActor() {
        return {
          async read_node(databaseId, path) {
            calls.push(["read", databaseId, path]);
            return { Ok: [{ etag: "etag-1" }] };
          },
          async write_node(request) {
            calls.push(["write", request.database_id, request.path, request.expected_etag]);
            return {
              Ok: {
                created: false,
                node: { etag: "etag-2" }
              }
            };
          }
        };
      }
    }
  );

  assert.deepEqual(calls, [
    ["read", "team-db", "/Sources/raw/chatgpt-abc/chatgpt-abc.md"],
    ["write", "team-db", "/Sources/raw/chatgpt-abc/chatgpt-abc.md", ["etag-1"]]
  ]);
  assert.equal(result.created, false);
  assert.equal(result.etag, "etag-2");
});

test("saveSource rejects missing database id", async () => {
  await assert.rejects(
    () =>
      saveSource(capture(), { canisterId: "aaaaa-aa", databaseId: "", host: "http://127.0.0.1:8001" }, { storage: memoryStorage() }),
    /database id is required/
  );
});

function capture() {
  return {
    provider: "chatgpt",
    url: "https://chatgpt.com/c/abc",
    capturedAt: "2026-05-01T00:00:00.000Z",
    conversationId: "abc",
    conversationTitle: "Project",
    messages: [{ role: "user", content: "Hello" }],
    captureMethod: "direct api"
  };
}

function memoryStorage() {
  const values = new Map();
  return {
    async get(defaults) {
      return { ...defaults, ...Object.fromEntries(values) };
    },
    async set(next) {
      for (const [key, value] of Object.entries(next)) {
        values.set(key, value);
      }
    }
  };
}
