// Where: extensions/conversation-capture/src/service-worker.js
// What: MV3 background workflow for canister persistence.
// Why: Content scripts fetch ChatGPT data while the worker owns canister writes.
import { buildRawSource } from "./raw-source.js";

const DEFAULT_CONFIG = {
  canisterId: "",
  databaseId: "",
  host: "https://icp0.io"
};

if (globalThis.chrome?.runtime?.onMessage) {
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    handleMessage(message, sender).then(sendResponse, (error) => {
      sendResponse({ ok: false, error: error instanceof Error ? error.message : String(error) });
    });
    return true;
  });
}

export async function handleMessage(message, sender) {
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

export async function saveSource(capture, overrideConfig, deps = {}) {
  const actorFactory = deps.createVfsActor || defaultCreateVfsActor;
  const config = { ...(await loadConfig(deps.storage)), ...(overrideConfig || {}) };
  if (!config.canisterId) {
    throw new Error("canister id is required");
  }
  if (!config.databaseId) {
    throw new Error("database id is required");
  }
  const raw = buildRawSource(capture);
  const actor = await actorFactory(config);
  const existing = await actor.read_node(config.databaseId, raw.path);
  if ("Err" in existing) {
    throw new Error(existing.Err);
  }
  const expected = existing.Ok[0]?.etag ? [existing.Ok[0].etag] : [];
  const result = await actor.write_node({
    database_id: config.databaseId,
    path: raw.path,
    kind: { Source: null },
    content: raw.content,
    metadata_json: raw.metadataJson,
    expected_etag: expected
  });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  await saveConfig(config, deps.storage);
  return {
    path: raw.path,
    sourceId: raw.sourceId,
    created: result.Ok.created,
    etag: result.Ok.node.etag
  };
}

async function defaultCreateVfsActor(config) {
  const { createVfsActor } = await import("./vfs-actor.js");
  return createVfsActor(config);
}

export async function loadConfig(storage = chrome.storage.sync) {
  const stored = await storage.get(DEFAULT_CONFIG);
  return {
    canisterId: String(stored.canisterId || ""),
    databaseId: String(stored.databaseId || ""),
    host: String(stored.host || DEFAULT_CONFIG.host)
  };
}

export async function saveConfig(config, storage = chrome.storage.sync) {
  await storage.set({
    canisterId: String(config?.canisterId || ""),
    databaseId: String(config?.databaseId || ""),
    host: String(config?.host || DEFAULT_CONFIG.host)
  });
}
