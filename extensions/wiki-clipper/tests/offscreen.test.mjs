// Where: extensions/wiki-clipper/tests/offscreen.test.mjs
// What: Unit tests for authenticated offscreen VFS writes.
// Why: Service workers delegate II-backed writes to offscreen documents.
import assert from "node:assert/strict";
import test from "node:test";
import { authStatus, saveRawSource, setOffscreenDepsForTest } from "../src/offscreen.js";

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
