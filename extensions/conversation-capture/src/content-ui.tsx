// Where: extensions/conversation-capture/src/content-ui.tsx
// What: Inject an on-demand recent-chat export modal into ChatGPT pages.
// Why: Users should explicitly export recent chats without switching the visible tab.
import { computed, signal } from "@preact/signals";
import { render } from "preact";
import { cancelCurrentTabExport, resumeCurrentTabExport, startCurrentTabExport } from "./current-tab-export.js";
import { DEFAULT_EXPORT_LIMIT, normalizeExportLimit } from "./history-links.js";

const ROOT_ID = "kinic-conversation-capture-root";
const config = signal({ canisterId: "", host: "https://icp0.io" });
const countText = signal(String(DEFAULT_EXPORT_LIMIT));
const status = signal("idle");
const error = signal("");
const panelOpen = signal(false);
const logs = signal([]);
const phase = signal("idle");
const progress = signal({ total: 0, done: 0, ok: 0, failed: 0 });
const canExport = computed(() => Boolean(config.value.canisterId && status.value !== "exporting"));
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
        Kinic Memory
      </button>
      {panelOpen.value ? <Modal /> : null}
    </>
  );
}

function Modal() {
  return (
    <section class="panel" aria-label="Kinic Memory export">
      <header class="panel-header">
        <div class="brand">
          <span class="brand-icon">K</span>
          <strong>Kinic</strong>
          <span class="pill">Memory</span>
        </div>
        <button class="close" type="button" onClick={() => (panelOpen.value = false)}>
          x
        </button>
      </header>
      <section class="settings">
        <label class="row">
          <span>Canister ID</span>
          <input value={config.value.canisterId} onInput={(event) => updateConfig({ canisterId: event.currentTarget.value })} />
        </label>
        <label class="row">
          <span>IC host</span>
          <input value={config.value.host} onInput={(event) => updateConfig({ host: event.currentTarget.value })} />
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
    await send({ type: "save-config", config: normalizedConfig() });
    await startCurrentTabExport({
      limit,
      config: normalizedConfig(),
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
    config.value = response.config;
  } catch (nextError) {
    error.value = messageForError(nextError);
  }
}

function updateConfig(patch) {
  config.value = { ...config.value, ...patch };
}

function normalizedConfig() {
  return { canisterId: config.value.canisterId.trim(), host: config.value.host.trim() || "https://icp0.io" };
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
      config.value = nextState.config || config.value;
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
:host{all:initial;color-scheme:dark;font-family:Inter,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
*{box-sizing:border-box}
.kinic-fab{position:fixed;right:18px;bottom:18px;z-index:2147483647;border:1px solid #515a85;border-radius:999px;padding:10px 14px;background:#20263f;color:#fff;font:700 13px/1 system-ui;box-shadow:0 12px 28px rgb(0 0 0 / 24%)}
.panel{position:fixed;right:18px;bottom:62px;z-index:2147483647;width:min(672px,calc(100vw - 32px));max-height:min(650px,calc(100vh - 86px));overflow:hidden;border:1px solid #465078;border-radius:14px;background:linear-gradient(135deg,#252d4b,#1e263a 54%,#302b39);color:#eef2ff;box-shadow:0 24px 60px rgb(0 0 0 / 40%);font:14px/1.42 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
.panel-header{display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid rgb(255 255 255 / 8%);padding:12px 18px}
.brand{display:flex;align-items:center;gap:8px}.brand-icon{display:grid;place-items:center;width:20px;height:20px;border-radius:50%;background:linear-gradient(135deg,#ffb06b,#7c4dff);font-size:11px;font-weight:800}.pill{border-radius:8px;padding:5px 8px;background:linear-gradient(135deg,#ff9ccf,#7c4dff);font-size:12px;font-weight:800}.close{border:0;background:transparent;color:#aeb6ce;font-size:18px}
.settings{margin:10px;border:1px solid rgb(255 255 255 / 9%);border-radius:10px;background:rgb(38 47 76 / 62%);padding:16px}.row{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:14px;font-weight:700}.row input{max-width:280px}
input{border:1px solid #56617f;border-radius:999px;background:#182032;color:#fff;padding:9px 12px;font:inherit}
.export-block{display:grid;gap:10px}.export-warning{margin:0;color:#ffd59f;font-weight:750}.export-box{display:flex;align-items:center;justify-content:space-between;gap:18px;border:1px solid #465371;border-radius:9px;padding:16px}.export-box p{max-width:430px;margin:0;color:#b9c1d4;font-weight:600}.export-control{display:flex;align-items:center;gap:10px;border:1px solid #53607e;border-radius:999px;padding:5px;background:#151d2e}.export-control input{width:58px;border:0;background:transparent;text-align:center;font-weight:800}.export-control button,.logs button{border:0;border-radius:999px;padding:9px 14px;background:linear-gradient(135deg,#ff9ccf,#7c4dff);color:#fff;font-weight:800}
button:disabled{opacity:.55}.logs{margin:10px;border:1px solid rgb(255 255 255 / 8%);border-radius:10px;background:rgb(31 39 58 / 76%);padding:14px 20px}.logs h2{margin:0 0 12px;font-size:16px}.filter{border:4px solid #1a263a;border-radius:999px;background:#344154;padding:8px;text-align:center;font-weight:800}.status{min-height:20px;margin:10px 0;color:#b9c1d4}.status.error{color:#ffb4aa}.cancel{margin:0 0 10px;border:1px solid #59647f;border-radius:999px;padding:8px 12px;background:#243047;color:#fff;font-weight:800}.log-list{display:grid;gap:12px;max-height:240px;overflow:auto}.log{display:grid;grid-template-columns:42px 1fr;gap:12px;border:1px solid #4b5874;border-radius:10px;padding:14px;background:rgb(72 77 91 / 55%)}.log-icon{display:grid;place-items:center;width:40px;height:40px;border-radius:50%;background:#176349;color:#99e8c6;font-weight:900}.log.error .log-icon{background:#69302c;color:#ffd0ca}.log-meta{display:flex;justify-content:space-between;color:#b9c1d4;font-size:12px}.log p{margin:6px 0 0;color:#fff;font-weight:650}
`;
