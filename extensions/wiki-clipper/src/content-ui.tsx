// Where: extensions/wiki-clipper/src/content-ui.tsx
// What: Inject an on-demand recent-chat export modal into ChatGPT pages.
// Why: Users should explicitly export recent chats without switching the visible tab.
import { computed, signal } from "@preact/signals";
import { render } from "preact";
import { cancelCurrentTabExport, resumeCurrentTabExport, startCurrentTabExport } from "./current-tab-export.js";
import { DEFAULT_EXPORT_LIMIT, normalizeExportLimit } from "./history-links.js";
import { DEFAULT_CANISTER_ID, DEFAULT_IC_HOST } from "./url-ingest-request.js";

const ROOT_ID = "kinic-wiki-clipper-root";
const DEFAULT_DATABASE_ID = "";
const config = signal({ canisterId: DEFAULT_CANISTER_ID, databaseId: DEFAULT_DATABASE_ID, host: DEFAULT_IC_HOST });
const countText = signal(String(DEFAULT_EXPORT_LIMIT));
const status = signal("idle");
const error = signal("");
const panelOpen = signal(false);
const logs = signal([]);
const phase = signal("idle");
const progress = signal({ total: 0, done: 0, ok: 0, failed: 0 });
const canExport = computed(() => status.value !== "exporting");
let resumeStarted = false;

ensureMounted();
new MutationObserver(() => ensureMounted()).observe(document.documentElement, { childList: true, subtree: true });

function ensureMounted() {
  if (document.getElementById(ROOT_ID) || !document.body) return;
  const host = document.createElement("div");
  host.id = ROOT_ID;
  document.body.append(host);
  render(<App />, host.attachShadow({ mode: "open" }));
  loadConfig();
  resumeExport();
}

function App() {
  return (
    <>
      <style>{styles}</style>
      <button class="kinic-fab" type="button" onClick={() => (panelOpen.value = true)}>
        <span class="kinic-mark" aria-hidden="true">K</span>
        <span>Kinic Wiki Clipper</span>
      </button>
      {panelOpen.value ? <Modal /> : null}
    </>
  );
}

function Modal() {
  return (
    <section class="panel" aria-label="Kinic Wiki Clipper export">
      <header class="panel-header">
        <div class="brand">
          <span class="kinic-mark" aria-hidden="true">K</span>
          <div>
            <strong>Kinic Wiki Clipper</strong>
            <p>Export ChatGPT conversations into your wiki</p>
          </div>
          <span class="pill">ChatGPT export</span>
        </div>
        <button class="close" type="button" aria-label="Close" onClick={() => (panelOpen.value = false)}>
          x
        </button>
      </header>
      <section class="settings">
        <label class="row">
          <span>Database ID</span>
          <input value={config.value.databaseId} onInput={(event) => updateConfig({ databaseId: event.currentTarget.value })} />
        </label>
        <div class="export-block">
          <strong>Export the recent chats</strong>
          {status.value === "exporting" ? (
            <p class="export-warning">Export is running. You can keep using this tab, but do not close it until it finishes.</p>
          ) : null}
          <div class="export-box">
            <p>Processing takes ~10 seconds per chat. If you have over 50 chats, export manually to save time.</p>
            <div class="export-control">
              <input
                inputMode="numeric"
                value={countText.value}
                onInput={(event) => (countText.value = event.currentTarget.value)}
                onBlur={() => (countText.value = String(normalizeExportLimit(countText.value)))}
              />
              <button type="button" disabled={!canExport.value} onClick={startExport}>
                Export
              </button>
            </div>
          </div>
        </div>
      </section>
      <section class="logs">
        <h2>Logs</h2>
        <div class="filter">All</div>
        <p class={`status ${error.value ? "error" : ""}`}>{statusText()}</p>
        {status.value === "exporting" ? (
          <button class="cancel" type="button" onClick={cancelExport}>
            Stop export
          </button>
        ) : null}
        <div class="log-list">{logs.value.map((log) => <LogItem key={log.id} log={log} />)}</div>
      </section>
    </section>
  );
}

function LogItem({ log }) {
  return (
    <article class={`log ${log.kind}`}>
      <span class="log-icon">K</span>
      <div>
        <div class="log-meta">
          <span>{log.time}</span>
          <span>{log.provider || "ChatGPT"}</span>
        </div>
        <p>{log.message}</p>
      </div>
    </article>
  );
}

async function startExport() {
  error.value = "";
  logs.value = [];
  const limit = normalizeExportLimit(countText.value);
  countText.value = String(limit);
  status.value = "exporting";
  phase.value = "fetching";
  progress.value = { total: limit, done: 0, ok: 0, failed: 0 };
  try {
    const nextConfig = normalizedConfig();
    if (isMainnetHost(nextConfig.host) && !confirmMainnetExport()) {
      status.value = "idle";
      return;
    }
    await send({ type: "save-config", config: nextConfig });
    await startCurrentTabExport({
      limit,
      config: nextConfig,
      originalUrl: location.href,
      callbacks: exportCallbacks()
    });
  } catch (nextError) {
    error.value = messageForError(nextError);
    status.value = "error";
  }
}

async function cancelExport() {
  await cancelCurrentTabExport(exportCallbacks());
}

async function loadConfig() {
  try {
    const response = await send({ type: "load-config" });
    config.value = configWithDefaults(response.config);
  } catch (nextError) {
    error.value = messageForError(nextError);
  }
}

function updateConfig(patch) {
  config.value = { ...config.value, ...patch };
}

function configWithDefaults(value) {
  return {
    canisterId: String(value?.canisterId || DEFAULT_CANISTER_ID),
    databaseId: String(value?.databaseId || DEFAULT_DATABASE_ID),
    host: DEFAULT_IC_HOST
  };
}

function normalizedConfig() {
  return {
    canisterId: DEFAULT_CANISTER_ID,
    databaseId: config.value.databaseId.trim() || DEFAULT_DATABASE_ID,
    host: DEFAULT_IC_HOST
  };
}

function isMainnetHost(host) {
  try {
    const { hostname } = new URL(host);
    return hostname === "icp0.io" || hostname.endsWith(".icp0.io");
  } catch {
    return false;
  }
}

function confirmMainnetExport() {
  return globalThis.confirm(
    "This will write ChatGPT conversations to a mainnet IC host using your Internet Identity principal. Continue?"
  );
}

async function send(message) {
  const response = await chrome.runtime.sendMessage(message);
  if (!response?.ok) throw new Error(response?.error || "extension request failed");
  return response;
}

function resumeExport() {
  if (resumeStarted) return;
  resumeStarted = true;
  resumeCurrentTabExport(exportCallbacks()).catch((nextError) => {
    error.value = messageForError(nextError);
    status.value = "error";
  });
}

function exportCallbacks() {
  return {
    send,
    onState(nextState) {
      panelOpen.value = true;
      config.value = configWithDefaults(nextState.config || config.value);
      progress.value = nextState.progress;
      logs.value = nextState.logs || [];
      status.value = nextState.status;
      phase.value = nextState.phase || phase.value;
      error.value = nextState.error || "";
    }
  };
}

function statusText() {
  if (error.value) return error.value;
  const value = progress.value;
  if (status.value === "idle") return "Ready";
  if (status.value === "exporting" && phase.value === "fetching") return `Fetching conversations... 0/${value.total}.`;
  if (status.value === "exporting") return `Exporting sources ${value.done}/${value.total}. Success ${value.ok}, failed ${value.failed}.`;
  if (status.value === "done") return `Export complete. Success ${value.ok}.`;
  if (status.value === "partial") return `Export complete with errors. Success ${value.ok}, failed ${value.failed}.`;
  if (status.value === "cancelled") return `Export cancelled. Success ${value.ok}, failed ${value.failed}.`;
  return "Ready";
}

function messageForError(value) {
  return value instanceof Error ? value.message : String(value);
}

const styles = `
:host{all:initial;color-scheme:light;font-family:Inter,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
*{box-sizing:border-box}
.kinic-fab{position:fixed;right:18px;bottom:18px;z-index:2147483647;display:inline-flex;align-items:center;gap:8px;border:1px solid #162338;border-radius:999px;padding:9px 14px;background:#162338;color:#fff;font:700 13px/1 system-ui;box-shadow:0 14px 30px rgb(15 23 42 / 24%);transition:background .18s ease,transform .18s ease,box-shadow .18s ease}
.kinic-fab:hover{background:#2f3d4d;transform:translateY(-1px);box-shadow:0 18px 34px rgb(15 23 42 / 28%)}
.kinic-mark{display:grid;place-items:center;flex:0 0 auto;width:24px;height:24px;border-radius:8px;background:linear-gradient(135deg,#ec1877 0%,#ec1877 30%,#fcb12b 31%,#23a8e0 62%,#f05c21 100%);color:#fff;font-size:12px;font-weight:900;line-height:1;box-shadow:inset 0 0 0 1px rgb(255 255 255 / 30%)}
.panel{position:fixed;right:18px;bottom:62px;z-index:2147483647;width:min(672px,calc(100vw - 32px));max-height:min(650px,calc(100vh - 86px));overflow:hidden;border:1px solid #e2e8f0;border-radius:14px;background:#fff;color:#0f172a;box-shadow:0 24px 60px rgb(15 23 42 / 22%);font:14px/1.42 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
.panel-header{display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #e2e8f0;padding:14px 18px;background:#fff}
.brand{display:flex;align-items:center;gap:10px;min-width:0}.brand strong{display:block;font-size:15px}.brand p{margin:2px 0 0;color:#64748b;font-size:12px;font-weight:550}.pill{border:1px solid #e2e8f0;border-radius:999px;padding:5px 8px;background:#f8fafc;color:#475569;font-size:12px;font-weight:800}.close{display:grid;place-items:center;width:30px;height:30px;border:1px solid #e2e8f0;border-radius:10px;background:#fff;color:#64748b;font-size:17px;font-weight:800}
.close:hover{background:#f1f5f9;color:#162338}
.settings{margin:12px;border:1px solid #e2e8f0;border-radius:12px;background:#f8fafc;padding:16px}.row{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:14px;color:#162338;font-weight:750}.row input{max-width:280px}
input{border:1px solid #cbd5e1;border-radius:10px;background:#fff;color:#0f172a;padding:9px 12px;font:inherit}
input:focus{border-color:#3b4c5f;outline:2px solid rgb(59 76 95 / 18%);outline-offset:1px}
.export-block{display:grid;gap:10px}.export-block strong{font-size:15px}.export-warning{margin:0;color:#b45309;font-weight:750}.export-box{display:flex;align-items:center;justify-content:space-between;gap:18px;border:1px solid #e2e8f0;border-radius:12px;padding:16px;background:#fff}.export-box p{max-width:430px;margin:0;color:#64748b;font-weight:600}.export-control{display:flex;align-items:center;gap:8px;border:1px solid #cbd5e1;border-radius:12px;padding:5px;background:#f8fafc}.export-control input{width:58px;border:0;background:transparent;text-align:center;font-weight:800}.export-control button,.logs button{border:0;border-radius:10px;padding:9px 14px;background:#162338;color:#fff;font-weight:800;box-shadow:0 8px 18px rgb(22 35 56 / 18%)}
.export-control button:hover,.logs button:hover{background:#2f3d4d}
button:disabled{opacity:.55;cursor:not-allowed}.logs{margin:12px;border:1px solid #e2e8f0;border-radius:12px;background:#fff;padding:14px 18px}.logs h2{margin:0 0 12px;font-size:16px}.filter{border:1px solid #e2e8f0;border-radius:999px;background:#f8fafc;color:#475569;padding:8px;text-align:center;font-weight:800}.status{min-height:20px;margin:10px 0;color:#64748b}.status.error{color:#b42318}.cancel{margin:0 0 10px;border:1px solid #cbd5e1;border-radius:10px;padding:8px 12px;background:#fff;color:#162338;font-weight:800;box-shadow:none}.log-list{display:grid;gap:12px;max-height:240px;overflow:auto}.log{display:grid;grid-template-columns:42px 1fr;gap:12px;border:1px solid #e2e8f0;border-radius:12px;padding:14px;background:#f8fafc}.log-icon{display:grid;place-items:center;width:40px;height:40px;border-radius:12px;background:#dcfce7;color:#166534;font-weight:900}.log.error .log-icon{background:#fee2e2;color:#991b1b}.log-meta{display:flex;justify-content:space-between;color:#64748b;font-size:12px}.log p{margin:6px 0 0;color:#0f172a;font-weight:650}
`;
