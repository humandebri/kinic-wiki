// Where: extensions/wiki-clipper/src/offscreen.js
// What: DOM-backed authenticated URL ingest worker for the MV3 extension.
// Why: Internet Identity AuthClient requires a window-like context, not the service worker.
import { authSnapshot as defaultAuthSnapshot } from "./auth-client.js";
import { buildUrlIngestRequest } from "./url-ingest-request.js";
import { createVfsActor as defaultCreateVfsActor } from "./vfs-actor.js";

const URL_INGEST_TRIGGER_URL = "https://wiki.kinic.xyz/api/url-ingest/trigger";

let authSnapshotFactory = defaultAuthSnapshot;
let vfsActorFactory = defaultCreateVfsActor;
let fetchFactory = (...args) => fetch(...args);

if (globalThis.chrome?.runtime?.onMessage) {
  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.target !== "offscreen") return false;
    const task =
      message?.type === "queue-url-ingest"
        ? queueUrlIngest(message.tab, message.config)
        : message?.type === "save-raw-source"
          ? saveRawSource(message.rawSource, message.config)
          : message?.type === "auth-status"
            ? authStatus()
            : null;
    if (!task) return false;
    task.then(
      (result) => sendResponse({ ok: true, result }),
      (error) => sendResponse({ ok: false, error: error instanceof Error ? error.message : String(error) })
    );
    return true;
  });
}

export async function queueUrlIngest(tab, config) {
  if (!tab?.url) throw new Error("active tab URL is required");
  if (!config?.canisterId) throw new Error("canister id is required");
  if (!config?.databaseId) throw new Error("database id is required");
  const snapshot = await authenticatedSnapshot();
  const request = buildUrlIngestRequest({
    url: tab.url,
    requestedBy: snapshot.principal
  });
  const actor = await vfsActorFactory({ ...config, identity: snapshot.identity });
  const result = await actor.write_node({
    database_id: config.databaseId,
    path: request.writeRequest.path,
    kind: request.writeRequest.kind,
    content: request.writeRequest.content,
    metadata_json: request.writeRequest.metadataJson,
    expected_etag: request.writeRequest.expectedEtag
  });
  if ("Err" in result) throw new Error(result.Err);
  const nonce = crypto.randomUUID();
  const grant = await authorizeTriggerGrant(actor, config.databaseId, request.requestPath, nonce);
  if (!grant.ok) {
    return {
      requestPath: request.requestPath,
      url: tab.url,
      title: tab.title || "",
      principal: snapshot.principal,
      etag: result.Ok.node.etag,
      triggered: false,
      triggerError: grant.error
    };
  }
  const trigger = await triggerUrlIngest(config.databaseId, request.requestPath, nonce);
  return {
    requestPath: request.requestPath,
    url: tab.url,
    title: tab.title || "",
    principal: snapshot.principal,
    etag: result.Ok.node.etag,
    triggered: trigger.ok,
    triggerError: trigger.error
  };
}

export async function saveRawSource(rawSource, config) {
  if (!rawSource?.path) throw new Error("raw source path is required");
  if (typeof rawSource.content !== "string") throw new Error("raw source content is required");
  if (typeof rawSource.metadataJson !== "string") throw new Error("raw source metadata is required");
  if (!config?.canisterId) throw new Error("canister id is required");
  if (!config?.databaseId) throw new Error("database id is required");
  const snapshot = await authenticatedSnapshot();
  const actor = await vfsActorFactory({ ...config, identity: snapshot.identity });
  const existing = await actor.read_node(config.databaseId, rawSource.path);
  if ("Err" in existing) throw new Error(existing.Err);
  const expected = existing.Ok[0]?.etag ? [existing.Ok[0].etag] : [];
  const result = await actor.write_node({
    database_id: config.databaseId,
    path: rawSource.path,
    kind: { Source: null },
    content: rawSource.content,
    metadata_json: rawSource.metadataJson,
    expected_etag: expected
  });
  if ("Err" in result) throw new Error(result.Err);
  return {
    path: rawSource.path,
    sourceId: rawSource.sourceId || "",
    created: result.Ok.created,
    principal: snapshot.principal,
    etag: result.Ok.node.etag
  };
}

export async function authStatus() {
  const snapshot = await authSnapshotFactory();
  return {
    isAuthenticated: Boolean(snapshot.isAuthenticated),
    principal: snapshot.principal || null
  };
}

export function setOffscreenDepsForTest(deps = {}) {
  authSnapshotFactory = deps.authSnapshot || defaultAuthSnapshot;
  vfsActorFactory = deps.createVfsActor || defaultCreateVfsActor;
  fetchFactory = deps.fetch || ((...args) => fetch(...args));
}

async function authenticatedSnapshot() {
  const snapshot = await authSnapshotFactory();
  if (!snapshot.isAuthenticated || !snapshot.identity || !snapshot.principal) {
    throw new Error("UNAUTHENTICATED");
  }
  return snapshot;
}

async function authorizeTriggerGrant(actor, databaseId, requestPath, nonce) {
  try {
    const result = await actor.authorize_url_ingest_trigger({
      database_id: databaseId,
      request_path: requestPath,
      nonce
    });
    if ("Err" in result) {
      return { ok: false, error: result.Err };
    }
    return { ok: true, error: null };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : "worker trigger grant failed" };
  }
}

async function triggerUrlIngest(databaseId, requestPath, nonce) {
  try {
    const response = await fetchFactory(URL_INGEST_TRIGGER_URL, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ databaseId, requestPath, nonce })
    });
    if (!response.ok) {
      return { ok: false, error: `worker trigger failed: HTTP ${response.status}` };
    }
    return { ok: true, error: null };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : "worker trigger failed" };
  }
}
