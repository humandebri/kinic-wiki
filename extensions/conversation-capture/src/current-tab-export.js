// Where: extensions/conversation-capture/src/current-tab-export.js
// What: Export recent ChatGPT conversations through ChatGPT backend APIs.
// Why: Direct API export avoids visible tab navigation.
import { captureFromChatGptResponse } from "./chatgpt-response.js";

const STATE_KEY = "kinic-current-tab-export-v1";
const STATE_VERSION = 1;
const STATE_TTL_MS = 30 * 60 * 1000;
const FETCH_CONCURRENCY = 3;
const SAVE_CONCURRENCY = 2;
const CONVERSATION_LIST_PAGE_SIZE = 28;

export function createExportState({ limit, config, originalUrl, startedAt = new Date().toISOString() }) {
  const startedAtMs = Date.parse(startedAt);
  const expiresAt = new Date((Number.isFinite(startedAtMs) ? startedAtMs : Date.now()) + STATE_TTL_MS).toISOString();
  return {
    version: STATE_VERSION,
    limit,
    targets: [],
    config,
    originalUrl,
    startedAt,
    expiresAt,
    phase: "fetching",
    status: "exporting",
    error: "",
    progress: { total: limit, done: 0, ok: 0, failed: 0 },
    logs: []
  };
}

export function readExportState(storage = globalThis.sessionStorage) {
  try {
    if (!storage) return null;
    const raw = storage.getItem(STATE_KEY);
    const state = raw ? JSON.parse(raw) : null;
    if (!isValidState(state)) {
      if (raw) storage.removeItem(STATE_KEY);
      return null;
    }
    return state;
  } catch {
    storage.removeItem(STATE_KEY);
    return null;
  }
}

export function writeExportState(state, storage = globalThis.sessionStorage) {
  if (!storage) return;
  storage.setItem(STATE_KEY, JSON.stringify(state));
}

export function clearExportState(storage = globalThis.sessionStorage) {
  if (!storage) return;
  storage.removeItem(STATE_KEY);
}

export async function startCurrentTabExport(options) {
  const state = createExportState(options);
  writeExportState(state);
  await processDirectExport(options.callbacks);
}

export async function resumeCurrentTabExport(callbacks) {
  const state = readExportState();
  if (!state) return;
  hydrate(callbacks, state);
  if (state.status !== "exporting") {
    clearExportState();
    return;
  }
  const next = { ...state, status: "cancelled", error: "Previous export was interrupted." };
  writeExportState(next);
  hydrate(callbacks, next);
  clearExportState();
}

export async function cancelCurrentTabExport(callbacks) {
  const state = readExportState();
  if (!state) return;
  const next = { ...state, status: "cancelled" };
  writeExportState(next);
  hydrate(callbacks, next);
}

async function processDirectExport(callbacks) {
  let state = readExportState();
  if (!state || state.status !== "exporting") return;
  hydrate(callbacks, state);

  try {
    const targets = await fetchRecentConversationTargets(state.limit);
    state = {
      ...(readExportState() || state),
      targets,
      phase: "saving",
      progress: { total: targets.length, done: 0, ok: 0, failed: 0 }
    };
    writeExportState(state);
    hydrate(callbacks, state);
    if (state.status === "cancelled") {
      clearExportState();
      return;
    }
    if (!targets.length) {
      throw new Error("No recent ChatGPT conversations found.");
    }

    const captures = await mapWithConcurrency(targets, FETCH_CONCURRENCY, async (target) => fetchConversationCapture(target));
    if (readExportState()?.status === "cancelled") {
      hydrate(callbacks, readExportState());
      clearExportState();
      return;
    }
    let latest = readExportState() || state;
    await mapWithConcurrency(captures, SAVE_CONCURRENCY, async (result) => {
      const event = await saveCaptureResult(result, state.config, callbacks.send);
      latest = advanceState(readExportState() || latest, event);
      writeExportState(latest);
      hydrate(callbacks, latest);
      return event;
    });
    if (readExportState()?.status === "cancelled") {
      hydrate(callbacks, readExportState());
      clearExportState();
      return;
    }
    state = readExportState() || latest;
    state = { ...state, status: state.progress.failed ? "partial" : "done" };
    writeExportState(state);
    hydrate(callbacks, state);
    clearExportState();
  } catch (error) {
    const failed = {
      ...(readExportState() || state),
      status: "error",
      error: error instanceof Error ? error.message : String(error)
    };
    writeExportState(failed);
    hydrate(callbacks, failed);
    clearExportState();
  }
}

export async function fetchRecentConversationTargets(limit, fetchImpl = fetch, loc = location) {
  const targets = [];
  const seen = new Set();
  let offset = 0;
  while (targets.length < limit) {
    const pageLimit = Math.min(CONVERSATION_LIST_PAGE_SIZE, limit - targets.length);
    const payload = await fetchJson(`/backend-api/conversations?offset=${offset}&limit=${pageLimit}&order=updated`, fetchImpl);
    const items = Array.isArray(payload?.items) ? payload.items : Array.isArray(payload) ? payload : [];
    if (!items.length) break;
    for (const item of items) {
      const id = conversationIdFromListItem(item);
      if (!id || seen.has(id)) continue;
      seen.add(id);
      targets.push({
        id,
        title: titleFromListItem(item),
        url: new URL(`/c/${id}`, loc.origin).toString()
      });
      if (targets.length >= limit) break;
    }
    offset += items.length;
    if (payload?.has_more === false) break;
  }
  return targets;
}

export async function fetchConversationCapture(target, fetchImpl = fetch) {
  try {
    const payload = await fetchJson(`/backend-api/conversation/${encodeURIComponent(target.id)}`, fetchImpl);
    const capture = captureFromChatGptResponse(payload, target.url);
    capture.captureMethod = "direct api";
    if (!payload?.mapping || !payload?.current_node || capture.messages.length === 0) {
      return { ok: false, target, error: "no conversation messages found" };
    }
    return { ok: true, target, capture };
  } catch (error) {
    return { ok: false, target, error: error instanceof Error ? error.message : String(error) };
  }
}

async function saveCaptureResult(result, config, send) {
  if (exportIsCancelled()) {
    return { ok: false, title: result.target.title, url: result.target.url, error: "export cancelled" };
  }
  if (!result.ok) {
    return { ok: false, title: result.target.title, url: result.target.url, error: result.error };
  }
  try {
    const response = await send({ type: "save-source", capture: result.capture, config });
    return {
      ok: true,
      title: result.capture.conversationTitle || result.target.title,
      provider: result.capture.provider,
      captureMethod: result.capture.captureMethod,
      path: response.result.path,
      created: response.result.created
    };
  } catch (error) {
    return {
      ok: false,
      title: result.target.title,
      url: result.target.url,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

export function advanceState(state, event) {
  const progress = {
    ...state.progress,
    done: state.progress.done + 1,
    ok: state.progress.ok + (event.ok ? 1 : 0),
    failed: state.progress.failed + (event.ok ? 0 : 1)
  };
  return {
    ...state,
    progress,
    logs: [logFromEvent(event), ...state.logs].slice(0, 100)
  };
}

export function isValidState(state, now = new Date()) {
  if (!state || typeof state !== "object") return false;
  if (state.version !== STATE_VERSION) return false;
  if (typeof state.limit !== "number" || !Array.isArray(state.targets)) return false;
  if (!state.progress || typeof state.progress !== "object") return false;
  const expiresAtMs = Date.parse(state.expiresAt);
  if (!Number.isFinite(expiresAtMs) || expiresAtMs <= now.getTime()) return false;
  if (!["exporting", "cancelled", "done", "partial", "error"].includes(state.status)) return false;
  return true;
}

export async function mapWithConcurrency(items, concurrency, worker) {
  const results = new Array(items.length);
  let nextIndex = 0;
  async function run() {
    for (;;) {
      if (exportIsCancelled()) return;
      const index = nextIndex;
      nextIndex += 1;
      if (index >= items.length) return;
      results[index] = await worker(items[index], index);
    }
  }
  const workers = Array.from({ length: Math.min(concurrency, items.length) }, () => run());
  await Promise.all(workers);
  return results.filter(Boolean);
}

function exportIsCancelled() {
  return readExportState()?.status === "cancelled";
}

async function fetchJson(url, fetchImpl) {
  const response = await fetchImpl(url, {
    method: "GET",
    credentials: "include",
    headers: { Accept: "application/json" }
  });
  if (!response.ok) {
    throw new Error(`ChatGPT API failed: ${response.status}`);
  }
  return response.json();
}

function conversationIdFromListItem(item) {
  if (typeof item?.id === "string") return item.id;
  if (typeof item?.conversation_id === "string") return item.conversation_id;
  return "";
}

function titleFromListItem(item) {
  const title = typeof item?.title === "string" ? item.title.trim() : "";
  return title || "Untitled conversation";
}

function hydrate(callbacks, state) {
  callbacks.onState?.(state);
}

function logFromEvent(event) {
  return {
    id: `${event.title || event.url}-${Date.now()}`,
    kind: event.ok ? "success" : "error",
    provider: event.provider || "ChatGPT",
    time: "just now",
    message: event.ok
      ? `Memory: ${event.title || event.path} via ${event.captureMethod || "unknown"} (${event.created ? "Created" : "Updated"})`
      : `Failed: ${event.title || event.url} - ${event.error}`
  };
}
