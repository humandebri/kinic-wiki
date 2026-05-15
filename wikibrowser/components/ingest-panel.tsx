"use client";

import type { Identity } from "@icp-sdk/core/agent";
import type { FormEvent } from "react";
import { useEffect, useState } from "react";
import { createUrlIngestRequest, ensureUrlIngestTriggerSession } from "@/lib/url-ingest";

type TriggerSessionState = "checking" | "ready" | "denied";
type TriggerSessionResult = {
  key: string;
  state: Exclude<TriggerSessionState, "checking">;
  error: string | null;
};

export function IngestPanel({
  canisterId,
  databaseId,
  readIdentity
}: {
  canisterId: string;
  databaseId: string;
  readIdentity: Identity | null;
}) {
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [tone, setTone] = useState<"error" | "info">("info");
  const sessionPrincipal = readIdentity?.getPrincipal().toText() ?? "";
  const triggerSessionKey = readIdentity ? `${canisterId}\n${databaseId}\n${sessionPrincipal}` : "";
  const [triggerSessionResult, setTriggerSessionResult] = useState<TriggerSessionResult | null>(null);
  const currentTriggerSessionResult = triggerSessionResult?.key === triggerSessionKey ? triggerSessionResult : null;
  const triggerSessionState: TriggerSessionState = currentTriggerSessionResult?.state ?? "checking";
  const sessionError = currentTriggerSessionResult?.error ?? null;

  useEffect(() => {
    let cancelled = false;
    if (!readIdentity) return;
    const key = triggerSessionKey;
    ensureUrlIngestTriggerSession(canisterId, databaseId, readIdentity)
      .then(() => {
        if (!cancelled) setTriggerSessionResult({ key, state: "ready", error: null });
      })
      .catch((cause) => {
        if (cancelled) return;
        setTriggerSessionResult({
          key,
          state: "denied",
          error: cause instanceof Error ? cause.message : "URL ingest access denied."
        });
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId, databaseId, readIdentity, triggerSessionKey]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!readIdentity || !url.trim() || triggerSessionState !== "ready") return;
    setBusy(true);
    setMessage(null);
    try {
      const created = await createUrlIngestRequest(canisterId, databaseId, readIdentity, url);
      setTriggerSessionResult({ key: triggerSessionKey, state: "ready", error: null });
      setTone(created.triggered ? "info" : "error");
      setMessage(created.triggered ? `Queued and accepted ${created.requestPath}` : `Queued ${created.requestPath}. ${created.triggerError}`);
      if (created.triggerError?.includes("HTTP 403")) {
        setTriggerSessionResult({ key: triggerSessionKey, state: "denied", error: created.triggerError });
      }
      setUrl("");
    } catch (cause) {
      setTone("error");
      setMessage(cause instanceof Error ? cause.message : "URL ingest failed.");
    } finally {
      setBusy(false);
    }
  }

  if (!readIdentity) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-3 p-4 text-sm">
        <div className="rounded-xl border border-line bg-white p-4">
          <p className="text-muted">Login is required to queue URL ingest for this database.</p>
        </div>
      </div>
    );
  }

  const triggerReady = triggerSessionState === "ready";
  const submitDisabled = busy || !url.trim() || !triggerReady;
  const displayMessage = message ?? sessionError;
  const displayTone = message ? tone : "error";

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 p-4 text-sm">
      <form className="grid gap-3" onSubmit={submit}>
        <div>
          <label className="text-xs uppercase tracking-[0.12em] text-muted" htmlFor="ingest-url">
            URL
          </label>
          <input
            id="ingest-url"
            className="mt-2 w-full rounded-lg border border-line bg-white px-3 py-2 text-sm outline-none focus:border-accent"
            placeholder="https://example.com/article"
            value={url}
            disabled={!triggerReady || busy}
            onChange={(event) => setUrl(event.target.value)}
          />
        </div>
        <button
          className="rounded-2xl border border-action bg-action px-3 py-2 text-sm font-bold text-white hover:-translate-y-[3px] hover:border-accent hover:bg-accent disabled:cursor-not-allowed disabled:translate-y-0 disabled:opacity-60"
          disabled={submitDisabled}
          type="submit"
        >
          {busy ? "Queueing..." : triggerSessionState === "checking" ? "Checking access..." : triggerSessionState === "denied" ? "URL ingest disabled" : "Queue URL"}
        </button>
      </form>
      <div className="rounded-lg border border-line bg-white px-3 py-2 font-mono text-xs text-muted">{databaseId}</div>
      {displayMessage ? <div className={`rounded-lg border px-3 py-2 text-xs ${displayTone === "error" ? "border-red-200 bg-red-50 text-red-900" : "border-line bg-white text-ink"}`}>{displayMessage}</div> : null}
    </div>
  );
}
