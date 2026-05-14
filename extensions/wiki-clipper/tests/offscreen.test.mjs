// Where: extensions/wiki-clipper/tests/offscreen.test.mjs
// What: Unit tests for authenticated offscreen VFS writes.
// Why: Service workers delegate II-backed writes to offscreen documents.
import assert from "node:assert/strict";
import test from "node:test";
import { authStatus, queueUrlIngest, saveRawSource, setOffscreenDepsForTest } from "../src/offscreen.js";

test("queueUrlIngest writes request and triggers via wiki route", async () => {
  const calls = [];
  const triggerCalls = [];
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: true, identity: { tag: "identity" }, principal: "principal-1" }),
    fetch: async (url, init) => {
      triggerCalls.push([url, init]);
      return Response.json({ accepted: true });
    },
    createVfsActor: async (config) => {
      calls.push(["create", config.identity, config.databaseId]);
      return {
        async write_node(request) {
          calls.push(["write", request.database_id, request.path, request.kind]);
          return { Ok: { created: true, node: { etag: "etag-request" } } };
        },
        async authorize_url_ingest_trigger(request) {
          calls.push(["grant", request.database_id, request.request_path, request.nonce]);
          return { Ok: null };
        }
      };
    }
  });
  try {
    const result = await queueUrlIngest({ url: "https://example.com/#x", title: "Example" }, config());

    assert.equal(result.etag, "etag-request");
    assert.equal(result.principal, "principal-1");
    assert.equal(result.triggered, true);
    assert.equal(result.triggerError, null);
    assert.deepEqual(calls[0], ["create", { tag: "identity" }, "team-db"]);
    assert.equal(calls[1][0], "write");
    assert.equal(calls[1][1], "team-db");
    assert.match(calls[1][2], /^\/Sources\/ingest-requests\/.+\.md$/);
    assert.deepEqual(calls[1][3], { File: null });
    assert.equal(calls[2][0], "grant");
    assert.equal(calls[2][1], "team-db");
    assert.equal(calls[2][2], result.requestPath);
    assert.equal(typeof calls[2][3], "string");
    assert.equal(triggerCalls[0][0], "https://wiki.kinic.xyz/api/url-ingest/trigger");
    assert.equal(triggerCalls[0][1].method, "POST");
    assert.deepEqual(JSON.parse(triggerCalls[0][1].body), {
      databaseId: "team-db",
      requestPath: result.requestPath,
      nonce: calls[2][3]
    });
  } finally {
    setOffscreenDepsForTest();
  }
});

test("queueUrlIngest keeps request result when trigger fails", async () => {
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: true, identity: { tag: "identity" }, principal: "principal-1" }),
    fetch: async () => new Response("nope", { status: 502 }),
    createVfsActor: async () => ({
      async write_node() {
        return { Ok: { created: true, node: { etag: "etag-request" } } };
      },
      async authorize_url_ingest_trigger() {
        return { Ok: null };
      }
    })
  });
  try {
    const result = await queueUrlIngest({ url: "https://example.com/#x", title: "Example" }, config());

    assert.equal(result.etag, "etag-request");
    assert.equal(result.triggered, false);
    assert.equal(result.triggerError, "worker trigger failed: HTTP 502");
  } finally {
    setOffscreenDepsForTest();
  }
});

test("queueUrlIngest keeps request result when grant authorize fails", async () => {
  const triggerCalls = [];
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: true, identity: { tag: "identity" }, principal: "principal-1" }),
    fetch: async (url, init) => {
      triggerCalls.push([url, init]);
      return Response.json({ accepted: true });
    },
    createVfsActor: async () => ({
      async write_node() {
        return { Ok: { created: true, node: { etag: "etag-request" } } };
      },
      async authorize_url_ingest_trigger() {
        return { Err: "caller mismatch" };
      }
    })
  });
  try {
    const result = await queueUrlIngest({ url: "https://example.com/#x", title: "Example" }, config());

    assert.equal(result.etag, "etag-request");
    assert.equal(result.triggered, false);
    assert.equal(result.triggerError, "caller mismatch");
    assert.equal(triggerCalls.length, 0);
  } finally {
    setOffscreenDepsForTest();
  }
});

test("saveRawSource writes with authenticated identity", async () => {
  const calls = [];
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: true, identity: { tag: "identity" }, principal: "principal-1" }),
    createVfsActor: async (config) => {
      calls.push(["create", config.identity, config.databaseId]);
      return {
        async read_node(databaseId, path) {
          calls.push(["read", databaseId, path]);
          return { Ok: [{ etag: "etag-1" }] };
        },
        async write_node(request) {
          calls.push(["write", request.database_id, request.path, request.expected_etag]);
          return { Ok: { created: false, node: { etag: "etag-2" } } };
        }
      };
    }
  });
  try {
    const result = await saveRawSource(rawSource(), config());

    assert.equal(result.etag, "etag-2");
    assert.equal(result.principal, "principal-1");
    assert.deepEqual(calls, [
      ["create", { tag: "identity" }, "team-db"],
      ["read", "team-db", "/Sources/raw/chatgpt-abc/chatgpt-abc.md"],
      ["write", "team-db", "/Sources/raw/chatgpt-abc/chatgpt-abc.md", ["etag-1"]]
    ]);
  } finally {
    setOffscreenDepsForTest();
  }
});

test("saveRawSource rejects unauthenticated sessions", async () => {
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: false, identity: null, principal: null })
  });
  try {
    await assert.rejects(() => saveRawSource(rawSource(), config()), /UNAUTHENTICATED/);
  } finally {
    setOffscreenDepsForTest();
  }
});

test("authStatus returns principal without identity", async () => {
  setOffscreenDepsForTest({
    authSnapshot: async () => ({ isAuthenticated: true, identity: { secret: "identity" }, principal: "principal-1" })
  });
  try {
    const result = await authStatus();

    assert.deepEqual(result, { isAuthenticated: true, principal: "principal-1" });
    assert.equal("identity" in result, false);
  } finally {
    setOffscreenDepsForTest();
  }
});

function rawSource() {
  return {
    path: "/Sources/raw/chatgpt-abc/chatgpt-abc.md",
    sourceId: "chatgpt-abc",
    content: "# ChatGPT",
    metadataJson: "{}"
  };
}

function config() {
  return {
    canisterId: "xis3j-paaaa-aaaai-axumq-cai",
    databaseId: "team-db",
    host: "https://icp0.io"
  };
}
