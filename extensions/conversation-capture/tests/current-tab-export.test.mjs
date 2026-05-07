// Where: extensions/conversation-capture/tests/current-tab-export.test.mjs
// What: Unit tests for direct ChatGPT API export.
// Why: Export must avoid tab navigation while keeping stable progress and logs.
import assert from "node:assert/strict";
import test from "node:test";
import {
  advanceState,
  createExportState,
  exportTarget,
  fetchConversationCapture,
  fetchRecentConversationTargets,
  isValidState,
  mapWithConcurrency,
  readExportState,
  writeExportState
} from "../src/current-tab-export.js";
import { handleMessage, validateSaveSource } from "../src/service-worker.js";

const VALID_CAPTURE = {
  provider: "chatgpt",
  conversationTitle: "Project",
  url: "https://chatgpt.com/c/abc",
  capturedAt: "2026-05-01T00:00:00.000Z",
  messages: [{ role: "user", content: "Hello" }]
};
const VALID_SENDER = { tab: { url: "https://chatgpt.com/c/abc" } };

test("createExportState initializes direct-api state", () => {
  const state = createExportState({
    limit: 10,
    config: { canisterId: "abc", host: "http://127.0.0.1:8001" },
    originalUrl: "https://chatgpt.com/",
    startedAt: "2026-05-01T00:00:00.000Z"
  });

  assert.equal(state.status, "exporting");
  assert.equal(state.phase, "fetching");
  assert.equal(state.version, 1);
  assert.equal(state.expiresAt, "2026-05-01T00:30:00.000Z");
  assert.deepEqual(state.progress, { total: 10, done: 0, ok: 0, failed: 0 });
});

test("fetchRecentConversationTargets paginates, dedupes, and limits", async () => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, init });
    if (url.includes("offset=0")) {
      return jsonResponse({
        items: [
          { id: "one", title: "One" },
          { conversation_id: "two", title: "Two" }
        ],
        has_more: true
      });
    }
    return jsonResponse({
      items: [
        { id: "two", title: "Two duplicate" },
        { id: "three", title: "Three" }
      ],
      has_more: false
    });
  };

  const targets = await fetchRecentConversationTargets(3, fetchImpl, { origin: "https://chatgpt.com" });

  assert.deepEqual(
    targets.map((target) => target.id),
    ["one", "two", "three"]
  );
  assert.equal(targets[2].url, "https://chatgpt.com/c/three");
  assert.equal(calls[0].init.credentials, "include");
  assert.match(calls[0].url, /offset=0&limit=3/);
});

test("fetchRecentConversationTargets surfaces API errors", async () => {
  await assert.rejects(
    () => fetchRecentConversationTargets(1, async () => jsonResponse({}, false, 500), { origin: "https://chatgpt.com" }),
    /ChatGPT API failed: 500/
  );
});

test("fetchConversationCapture converts payloads and rejects empty messages", async () => {
  const ok = await fetchConversationCapture(
    { id: "abc", title: "Project", url: "https://chatgpt.com/c/abc" },
    async () =>
      jsonResponse({
        conversation_id: "abc",
        title: "Project",
        current_node: "assistant1",
        mapping: {
          root: node(null, null),
          user1: node("root", message("user", "Hello")),
          assistant1: node("user1", message("assistant", "Hi"))
        }
      })
  );
  assert.equal(ok.ok, true);
  assert.equal(ok.capture.captureMethod, "direct api");
  assert.deepEqual(ok.capture.messages, [
    { role: "user", content: "Hello" },
    { role: "assistant", content: "Hi" }
  ]);

  const empty = await fetchConversationCapture(
    { id: "empty", title: "Empty", url: "https://chatgpt.com/c/empty" },
    async () => jsonResponse({ conversation_id: "empty", current_node: "root", mapping: { root: node(null, null) } })
  );
  assert.equal(empty.ok, false);
  assert.match(empty.error, /no conversation messages/);
});

test("exportTarget saves immediately after fetching a valid conversation", async () => {
  const calls = [];
  const target = { id: "abc", title: "Project", url: "https://chatgpt.com/c/abc" };
  const event = await exportTarget(
    target,
    { canisterId: "canister", host: "http://127.0.0.1:8001" },
    async (message) => {
      calls.push(["save", message.capture.conversationTitle]);
      return { result: { path: "/Sources/raw/chatgpt-abc/chatgpt-abc.md", created: true } };
    },
    async () => {
      calls.push(["fetch", target.id]);
      return jsonResponse(conversationPayload("abc", "Project"));
    }
  );

  assert.deepEqual(calls, [
    ["fetch", "abc"],
    ["save", "Project"]
  ]);
  assert.equal(event.ok, true);
  assert.equal(event.captureMethod, "direct api");
});

test("exportTarget does not save API failures or empty conversations", async () => {
  let saveCount = 0;
  const failed = await exportTarget(
    { id: "fail", title: "Fail", url: "https://chatgpt.com/c/fail" },
    {},
    async () => {
      saveCount += 1;
    },
    async () => jsonResponse({}, false, 500)
  );
  const empty = await exportTarget(
    { id: "empty", title: "Empty", url: "https://chatgpt.com/c/empty" },
    {},
    async () => {
      saveCount += 1;
    },
    async () => jsonResponse({ conversation_id: "empty", current_node: "root", mapping: { root: node(null, null) } })
  );

  assert.equal(saveCount, 0);
  assert.equal(failed.ok, false);
  assert.equal(empty.ok, false);
});

test("advanceState records direct-api success and errors in order", () => {
  let state = createExportState({
    limit: 2,
    config: {},
    originalUrl: "https://chatgpt.com/",
    startedAt: "2026-05-01T00:00:00.000Z"
  });
  state = { ...state, progress: { total: 2, done: 0, ok: 0, failed: 0 } };

  state = advanceState(state, { ok: true, title: "One", provider: "ChatGPT", captureMethod: "direct api", created: true });
  state = advanceState(state, { ok: false, title: "Two", url: "https://chatgpt.com/c/2", error: "empty" });

  assert.deepEqual(state.progress, { total: 2, done: 2, ok: 1, failed: 1 });
  assert.equal(state.logs[0].kind, "error");
  assert.match(state.logs[1].message, /via direct api/);
});

test("advanceState gives same-title same-timestamp logs unique ids", () => {
  const originalCrypto = globalThis.crypto;
  const originalDateNow = Date.now;
  let uuid = 0;
  Object.defineProperty(globalThis, "crypto", {
    configurable: true,
    value: { randomUUID: () => `uuid-${(uuid += 1)}` }
  });
  Date.now = () => 1;
  try {
    let state = createExportState({
      limit: 2,
      config: {},
      originalUrl: "https://chatgpt.com/",
      startedAt: "2026-05-01T00:00:00.000Z"
    });
    state = advanceState(state, { ok: true, title: "One", provider: "ChatGPT", captureMethod: "direct api", created: true });
    state = advanceState(state, { ok: true, title: "One", provider: "ChatGPT", captureMethod: "direct api", created: false });

    assert.notEqual(state.logs[0].id, state.logs[1].id);
  } finally {
    Date.now = originalDateNow;
    Object.defineProperty(globalThis, "crypto", { configurable: true, value: originalCrypto });
  }
});

test("readExportState removes stale and invalid state", async () => {
  const storage = memoryStorage();
  storage.setItem("kinic-current-tab-export-v1", JSON.stringify({ version: 1, expiresAt: "2026-05-01T00:00:00.000Z" }));

  assert.equal(await readExportState(storage), null);
  assert.equal(storage.getItem("kinic-current-tab-export-v1"), null);

  storage.setItem("kinic-current-tab-export-v1", JSON.stringify({ links: [] }));
  assert.equal(await readExportState(storage), null);
});

test("readExportState uses runtime-backed export state adapter by default", async () => {
  const restore = installRuntimeStorage(memoryStorage());
  try {
    const state = createExportState({
      limit: 1,
      config: {},
      originalUrl: "https://chatgpt.com/",
      startedAt: new Date().toISOString()
    });
    await writeExportState(state);

    assert.equal((await readExportState()).status, "exporting");
  } finally {
    restore();
  }
});

test("isValidState accepts non-expired schema v1 state", () => {
  const state = createExportState({
    limit: 1,
    config: {},
    originalUrl: "https://chatgpt.com/",
    startedAt: "2026-05-01T00:00:00.000Z"
  });

  assert.equal(isValidState(state, new Date("2026-05-01T00:29:00.000Z")), true);
  assert.equal(isValidState(state, new Date("2026-05-01T00:31:00.000Z")), false);
});

test("mapWithConcurrency does not exceed the configured limit", async () => {
  let active = 0;
  let maxActive = 0;
  const results = await mapWithConcurrency([1, 2, 3, 4, 5], 2, async (value) => {
    active += 1;
    maxActive = Math.max(maxActive, active);
    await new Promise((resolve) => setTimeout(resolve, 5));
    active -= 1;
    return value * 2;
  });

  assert.equal(maxActive, 2);
  assert.deepEqual(results, [2, 4, 6, 8, 10]);
});

test("validateSaveSource accepts ChatGPT captures from ChatGPT tabs", () => {
  assert.doesNotThrow(() => validateSaveSource(VALID_CAPTURE, VALID_SENDER));
  assert.doesNotThrow(() =>
    validateSaveSource(
      { ...VALID_CAPTURE, url: "https://chat.openai.com/c/abc" },
      { tab: { url: "https://chat.openai.com/c/abc" } }
    )
  );
});

test("validateSaveSource rejects non-ChatGPT senders and capture urls", () => {
  assert.throws(
    () => validateSaveSource(VALID_CAPTURE, { tab: { url: "https://evil.test/c/abc" } }),
    /sender must be a ChatGPT tab/
  );
  assert.throws(
    () => validateSaveSource({ ...VALID_CAPTURE, url: "https://evil.test/c/abc" }, VALID_SENDER),
    /capture url must be a ChatGPT conversation/
  );
});

test("validateSaveSource rejects wrong provider and malformed messages", () => {
  assert.throws(
    () => validateSaveSource({ ...VALID_CAPTURE, provider: "other" }, VALID_SENDER),
    /provider must be chatgpt/
  );
  assert.throws(
    () => validateSaveSource({ ...VALID_CAPTURE, messages: [] }, VALID_SENDER),
    /non-empty array/
  );
  assert.throws(
    () => validateSaveSource({ ...VALID_CAPTURE, messages: [{ role: "user", content: 1 }] }, VALID_SENDER),
    /string role and content/
  );
});

test("handleMessage stores export state in chrome.storage.session", async () => {
  const restore = installChromeStorageSession(memoryStorage());
  try {
    await handleMessage({ type: "export-state-set", key: "kinic-current-tab-export-v1", value: "state" }, {});
    assert.deepEqual(
      await handleMessage({ type: "export-state-get", key: "kinic-current-tab-export-v1" }, {}),
      { ok: true, value: "state" }
    );
    await handleMessage({ type: "export-state-remove", key: "kinic-current-tab-export-v1" }, {});
    assert.deepEqual(
      await handleMessage({ type: "export-state-get", key: "kinic-current-tab-export-v1" }, {}),
      { ok: true, value: null }
    );
  } finally {
    restore();
  }
});

test("handleMessage does not overwrite cancelled export state with stale progress", async () => {
  const restore = installChromeStorageSession(memoryStorage());
  try {
    await handleMessage({
      type: "export-state-set",
      key: "kinic-current-tab-export-v1",
      value: JSON.stringify({ status: "cancelled" })
    }, {});
    await handleMessage({
      type: "export-state-set",
      key: "kinic-current-tab-export-v1",
      value: JSON.stringify({ status: "exporting", progress: { done: 1 } })
    }, {});

    assert.deepEqual(
      await handleMessage({ type: "export-state-get", key: "kinic-current-tab-export-v1" }, {}),
      { ok: true, value: JSON.stringify({ status: "cancelled" }) }
    );
  } finally {
    restore();
  }
});

function jsonResponse(payload, ok = true, status = 200) {
  return {
    ok,
    status,
    async json() {
      return payload;
    }
  };
}

function conversationPayload(id, title) {
  return {
    conversation_id: id,
    title,
    current_node: "assistant1",
    mapping: {
      root: node(null, null),
      user1: node("root", message("user", "Hello")),
      assistant1: node("user1", message("assistant", "Hi"))
    }
  };
}

function node(parent, messageValue) {
  return { parent, children: [], message: messageValue };
}

function message(role, parts) {
  return {
    id: crypto.randomUUID(),
    author: { role },
    content: { parts: Array.isArray(parts) ? parts : [parts] }
  };
}

function memoryStorage() {
  const values = new Map();
  return {
    getItem(key) {
      return values.has(key) ? values.get(key) : null;
    },
    setItem(key, value) {
      values.set(key, String(value));
    },
    removeItem(key) {
      values.delete(key);
    }
  };
}

function installRuntimeStorage(storage) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "chrome");
  Object.defineProperty(globalThis, "chrome", {
    configurable: true,
    value: {
      runtime: {
        async sendMessage(message) {
          if (message.type === "export-state-get") {
            return { ok: true, value: storage.getItem(message.key) };
          }
          if (message.type === "export-state-set") {
            storage.setItem(message.key, message.value);
            return { ok: true };
          }
          if (message.type === "export-state-remove") {
            storage.removeItem(message.key);
            return { ok: true };
          }
          return { ok: false, error: "unknown message type" };
        }
      }
    }
  });
  return () => {
    if (descriptor) Object.defineProperty(globalThis, "chrome", descriptor);
    else delete globalThis.chrome;
  };
}

function installChromeStorageSession(storage) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "chrome");
  Object.defineProperty(globalThis, "chrome", {
    configurable: true,
    value: {
      storage: {
        session: {
          async get(key) {
            return { [key]: storage.getItem(key) };
          },
          async set(values) {
            for (const [key, value] of Object.entries(values)) {
              storage.setItem(key, value);
            }
          },
          async remove(key) {
            storage.removeItem(key);
          }
        }
      }
    }
  });
  return () => {
    if (descriptor) Object.defineProperty(globalThis, "chrome", descriptor);
    else delete globalThis.chrome;
  };
}
