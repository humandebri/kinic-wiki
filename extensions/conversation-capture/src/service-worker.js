// Where: extensions/conversation-capture/src/service-worker.js
// What: MV3 background workflow for canister persistence.
// Why: Content scripts fetch ChatGPT data while the worker owns canister writes.
import { buildRawSource } from "./raw-source.js";
import { createVfsActor } from "./vfs-actor.js";

const DEFAULT_CONFIG = {
  canisterId: "",
  host: "http://127.0.0.1:8001"
};
const ALLOWED_CHATGPT_ORIGINS = new Set(["https://chatgpt.com", "https://chat.openai.com"]);
const ALLOWED_MESSAGE_ROLES = new Set(["user", "assistant", "system"]);
const MAX_MESSAGE_COUNT = 500;
const MAX_MESSAGE_CONTENT_CHARS = 200_000;
const MAX_RAW_SOURCE_CHARS = 1_500_000;

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
  if (!isConversationUrl(capture.url)) {
    throw new Error("capture url must use /c/<id>");
  }
  if (capture.provider !== "chatgpt") {
    throw new Error("capture provider must be chatgpt");
  }
  if (typeof capture.conversationTitle !== "string") {
    throw new Error("capture conversationTitle must be a string");
  }
  if (!isIsoDateTime(capture.capturedAt)) {
    throw new Error("capture capturedAt must be an ISO timestamp");
  }
  if (!Array.isArray(capture.messages) || capture.messages.length === 0) {
    throw new Error("capture messages must be a non-empty array");
  }
  if (capture.messages.length > MAX_MESSAGE_COUNT) {
    throw new Error(`capture messages must not exceed ${MAX_MESSAGE_COUNT}`);
  }
  for (const message of capture.messages) {
    if (typeof message?.role !== "string" || typeof message?.content !== "string") {
      throw new Error("capture messages must contain string role and content");
    }
    if (!ALLOWED_MESSAGE_ROLES.has(message.role)) {
      throw new Error("capture message role must be user, assistant, or system");
    }
    if (message.content.length > MAX_MESSAGE_CONTENT_CHARS) {
      throw new Error(`capture message content must not exceed ${MAX_MESSAGE_CONTENT_CHARS} characters`);
    }
  }
  if (estimatedRawSourceSize(capture) > MAX_RAW_SOURCE_CHARS) {
    throw new Error(`capture raw source must not exceed ${MAX_RAW_SOURCE_CHARS} characters`);
  }
}

function isAllowedChatGptUrl(value) {
  try {
    return ALLOWED_CHATGPT_ORIGINS.has(new URL(value).origin);
  } catch {
    return false;
  }
}

function isConversationUrl(value) {
  try {
    return /^\/c\/[^/]+\/?$/.test(new URL(value).pathname);
  } catch {
    return false;
  }
}

function isIsoDateTime(value) {
  if (typeof value !== "string" || !value.includes("T")) return false;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp);
}

function estimatedRawSourceSize(capture) {
  return (
    String(capture.provider || "").length +
    String(capture.url || "").length +
    String(capture.capturedAt || "").length +
    String(capture.conversationTitle || "").length +
    capture.messages.reduce((total, message) => total + message.role.length + message.content.length + 64, 256)
  );
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
