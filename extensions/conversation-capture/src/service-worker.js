// Where: extensions/conversation-capture/src/service-worker.js
// What: MV3 background workflow for canister persistence.
// Why: Content scripts fetch ChatGPT data while the worker owns canister writes.
import { buildRawSource } from "./raw-source.js";
import { createVfsActor } from "./vfs-actor.js";

const DEFAULT_CONFIG = {
  canisterId: "",
  host: "https://icp0.io"
};

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  handleMessage(message, sender).then(sendResponse, (error) => {
    sendResponse({ ok: false, error: error instanceof Error ? error.message : String(error) });
  });
  return true;
});

async function handleMessage(message, sender) {
  if (message?.type === "save-source") {
    return { ok: true, result: await saveSource(message.capture, message.config) };
  }
  if (message?.type === "load-config") {
    return { ok: true, config: await loadConfig() };
  }
  if (message?.type === "save-config") {
    await saveConfig(message.config);
    return { ok: true };
  }
  return { ok: false, error: "unknown message type" };
}

async function saveSource(capture, overrideConfig) {
  const config = { ...(await loadConfig()), ...(overrideConfig || {}) };
  if (!config.canisterId) {
    throw new Error("canister id is required");
  }
  const raw = buildRawSource(capture);
  const actor = await createVfsActor(config);
  const existing = await actor.read_node(raw.path);
  if ("Err" in existing) {
    throw new Error(existing.Err);
  }
  const expected = existing.Ok[0]?.etag ? [existing.Ok[0].etag] : [];
  const result = await actor.write_node({
    path: raw.path,
    kind: { Source: null },
    content: raw.content,
    metadata_json: raw.metadataJson,
    expected_etag: expected
  });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  await saveConfig(config);
  return {
    path: raw.path,
    sourceId: raw.sourceId,
    created: result.Ok.created,
    etag: result.Ok.node.etag
  };
}

async function loadConfig() {
  const stored = await chrome.storage.sync.get(DEFAULT_CONFIG);
  return {
    canisterId: String(stored.canisterId || ""),
    host: String(stored.host || DEFAULT_CONFIG.host)
  };
}

async function saveConfig(config) {
  await chrome.storage.sync.set({
    canisterId: String(config?.canisterId || ""),
    host: String(config?.host || DEFAULT_CONFIG.host)
  });
}
