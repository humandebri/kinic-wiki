// Where: extensions/conversation-capture/src/service-worker.js
// What: MV3 background workflow for canister persistence.
// Why: Content scripts fetch ChatGPT data while the worker owns canister writes.
import { buildRawSource } from "./raw-source.js";
import { createVfsActor } from "./vfs-actor.js";

const DEFAULT_CONFIG = {
  canisterId: "",
  host: "https://icp0.io"
};
const ALLOWED_CHATGPT_ORIGINS = new Set(["https://chatgpt.com", "https://chat.openai.com"]);

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
    return { ok: true, result: await saveSource(message.capture, message.config, sender) };
  }
  if (message?.type === "load-config") {
    return { ok: true, config: await loadConfig() };
  }
  if (message?.type === "save-config") {
    await saveConfig(message.config);
    return { ok: true };
  }
  if (message?.type === "export-state-get") {
    return { ok: true, value: await readSessionValue(message.key) };
  }
  if (message?.type === "export-state-set") {
    await writeSessionValue(message.key, message.value);
    return { ok: true };
  }
  if (message?.type === "export-state-remove") {
    await removeSessionValue(message.key);
    return { ok: true };
  }
  return { ok: false, error: "unknown message type" };
}

async function saveSource(capture, overrideConfig, sender) {
  validateSaveSource(capture, sender);
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

export function validateSaveSource(capture, sender) {
  if (!isAllowedChatGptUrl(sender?.tab?.url)) {
    throw new Error("save-source sender must be a ChatGPT tab");
  }
  if (!isAllowedChatGptUrl(capture?.url)) {
    throw new Error("capture url must be a ChatGPT conversation");
  }
  if (capture.provider !== "chatgpt") {
    throw new Error("capture provider must be chatgpt");
  }
  if (!Array.isArray(capture.messages) || capture.messages.length === 0) {
    throw new Error("capture messages must be a non-empty array");
  }
  for (const message of capture.messages) {
    if (typeof message?.role !== "string" || typeof message?.content !== "string") {
      throw new Error("capture messages must contain string role and content");
    }
  }
}

function isAllowedChatGptUrl(value) {
  try {
    return ALLOWED_CHATGPT_ORIGINS.has(new URL(value).origin);
  } catch {
    return false;
  }
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

async function readSessionValue(key) {
  requireSessionKey(key);
  const values = await chrome.storage.session.get(key);
  return values?.[key] ?? null;
}

async function writeSessionValue(key, value) {
  requireSessionKey(key);
  const current = await readSessionValue(key);
  if (stateStatus(current) === "cancelled" && stateStatus(value) !== "cancelled") {
    return;
  }
  await chrome.storage.session.set({ [key]: String(value || "") });
}

async function removeSessionValue(key) {
  requireSessionKey(key);
  await chrome.storage.session.remove(key);
}

function requireSessionKey(key) {
  if (typeof key !== "string" || !key.startsWith("kinic-current-tab-export-")) {
    throw new Error("invalid export state key");
  }
}

function stateStatus(value) {
  try {
    return JSON.parse(value)?.status || "";
  } catch {
    return "";
  }
}
