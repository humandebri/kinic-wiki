"use client";

import type { Identity } from "@icp-sdk/core/agent";
import type { FormEvent, ReactNode } from "react";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { AlertTriangle, Clock3, Link2, Search, ShieldCheck } from "lucide-react";
import { RecentPanel } from "@/components/recent-panel";
import { createUrlIngestRequest } from "@/lib/url-ingest";
import { collectLintHints, type LintHint } from "@/lib/lint-hints";
import { classifyOpsInput, type OpsAction } from "@/lib/ops-actions";
import { collectOpsAnswerContext } from "@/lib/ops-context";
import { hrefForPath, hrefForSearch } from "@/lib/paths";
import type { ReadIdentityMode } from "@/lib/wiki-helpers";
import { errorMessage } from "@/lib/wiki-helpers";
import type { WikiNode } from "@/lib/types";
import { authorizeOpsAnswerSession, readNode } from "@/lib/vfs-client";

type OpsResult =
  | { kind: "message"; text: string; tone: "info" | "error" }
  | { kind: "lint"; targetPath: string; hints: LintHint[] }
  | { kind: "answer"; answer: string; citations: string[]; abstained: boolean };

export function OpsPanel({
  canisterId,
  databaseId,
  selectedPath,
  currentNode,
  readIdentity,
  writeIdentity,
  readMode,
  readIdentityMode
}: {
  canisterId: string;
  databaseId: string;
  selectedPath: string;
  currentNode: WikiNode | null;
  readIdentity: Identity | null;
  writeIdentity: Identity | null;
  readMode: "anonymous" | null;
  readIdentityMode: ReadIdentityMode;
}) {
  const router = useRouter();
  const [input, setInput] = useState("");
  const [activeAction, setActiveAction] = useState<OpsAction>({ kind: "recent", targetPath: null, sideEffect: "none", identityMode: readIdentityMode });
  const [pendingAction, setPendingAction] = useState<OpsAction | null>(null);
  const [result, setResult] = useState<OpsResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [lastAskAtMs, setLastAskAtMs] = useState(0);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const action = classifyOpsInput(input, selectedPath, readIdentityMode);
    if (!action) {
      setPendingAction(null);
      setResult({ kind: "message", tone: "error", text: "No supported operation found. Use recent, lint, search, or a URL." });
      return;
    }
    setInput("");
    await handleAction(action);
  }

  async function handleAction(action: OpsAction) {
    setResult(null);
    setActiveAction(action);
    if (action.kind === "queue_url") {
      setPendingAction(action);
      return;
    }
    setPendingAction(null);
    if (action.kind === "recent") return;
    if (action.kind === "search") {
      router.replace(hrefForSearch(canisterId, databaseId, action.query, "full", readMode));
      return;
    }
    if (action.kind === "ask") {
      await answerQuestion(action);
      return;
    }
    setBusy(true);
    try {
      const node = action.targetPath === selectedPath ? currentNode : await readNode(canisterId, databaseId, action.targetPath, readIdentity ?? undefined);
      if (!node) {
        setResult({ kind: "message", tone: "error", text: `Node not found: ${action.targetPath}` });
        return;
      }
      setResult({ kind: "lint", targetPath: action.targetPath, hints: collectLintHints(action.targetPath, node.content) });
    } catch (cause) {
      setResult({ kind: "message", tone: "error", text: errorMessage(cause) });
    } finally {
      setBusy(false);
    }
  }

  async function answerQuestion(action: Extract<OpsAction, { kind: "ask" }>) {
    if (!writeIdentity) {
      setResult({ kind: "message", tone: "error", text: "Login with Internet Identity to ask wiki questions." });
      return;
    }
    const now = Date.now();
    if (now - lastAskAtMs < 2_000) {
      setResult({ kind: "message", tone: "error", text: "Wait a moment before asking again." });
      return;
    }
    setLastAskAtMs(now);
    setBusy(true);
    try {
      const sessionNonce = crypto.randomUUID();
      await authorizeOpsAnswerSession(canisterId, writeIdentity, { databaseId, sessionNonce });
      const context = await collectOpsAnswerContext({ canisterId, databaseId, question: action.question, selectedPath, currentNode, readIdentity });
      const response = await fetch("/api/ops/answer", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ question: action.question, databaseId, selectedPath, sessionNonce, context })
      });
      const body: unknown = await response.json();
      if (!response.ok) {
        const message = isRecord(body) && typeof body.error === "string" ? body.error : `answer failed: HTTP ${response.status}`;
        throw new Error(message);
      }
      if (!isAnswerBody(body)) throw new Error("answer response shape is invalid");
      setResult({ kind: "answer", answer: body.answer, citations: body.citations, abstained: body.abstained });
    } catch (cause) {
      setResult({ kind: "message", tone: "error", text: errorMessage(cause) });
    } finally {
      setBusy(false);
    }
  }

  async function confirmQueueUrl(action: OpsAction) {
    if (action.kind !== "queue_url") return;
    if (!writeIdentity) {
      setResult({ kind: "message", tone: "error", text: "Login with Internet Identity to queue URL ingest." });
      return;
    }
    setBusy(true);
    try {
      const created = await createUrlIngestRequest(canisterId, databaseId, writeIdentity, action.url);
      setPendingAction(null);
      setResult({
        kind: "message",
        tone: created.triggered ? "info" : "error",
        text: created.triggered ? `Queued and started ${created.requestPath}` : `Queued ${created.requestPath}. ${created.triggerError}`
      });
    } catch (cause) {
      setResult({ kind: "message", tone: "error", text: errorMessage(cause) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden text-sm">
      <form className="border-b border-line p-3" onSubmit={submit}>
        <label className="text-xs uppercase tracking-[0.12em] text-muted" htmlFor="ops-command">
          Ops
        </label>
        <div className="mt-2 flex gap-2">
          <input
            id="ops-command"
            className="min-w-0 flex-1 rounded-lg border border-line bg-white px-3 py-2 text-sm outline-none focus:border-accent"
            placeholder="ask wiki, lint facts, search budget, https://..."
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />
          <button className="inline-flex items-center justify-center gap-1.5 rounded-lg border border-accent bg-accent px-3 text-sm font-medium text-white disabled:opacity-60" disabled={busy || !input.trim()} type="submit" title="Run operation">
            <ShieldCheck size={15} />
            <span>Run</span>
          </button>
        </div>
        <div className="mt-3 grid grid-cols-2 gap-2">
          <OpsButton icon={<Clock3 size={14} />} label="Recent" onClick={() => void handleAction({ kind: "recent", targetPath: null, sideEffect: "none", identityMode: readIdentityMode })} />
          <OpsButton icon={<AlertTriangle size={14} />} label="Lint current" onClick={() => void handleAction({ kind: "lint", targetPath: selectedPath, sideEffect: "none", identityMode: readIdentityMode })} />
          <OpsButton icon={<AlertTriangle size={14} />} label="Lint facts" onClick={() => void handleAction({ kind: "lint", targetPath: "/Wiki/facts.md", sideEffect: "none", identityMode: readIdentityMode })} />
          <OpsButton icon={<Search size={14} />} label="Search" onClick={() => input.trim() ? void handleAction({ kind: "search", targetPath: "/Wiki", sideEffect: "none", identityMode: readIdentityMode, query: input.trim() }) : setResult({ kind: "message", tone: "error", text: "Enter a search query." })} />
        </div>
      </form>
      <ActionPreview action={pendingAction ?? activeAction} busy={busy} onConfirm={pendingAction ? () => void confirmQueueUrl(pendingAction) : null} />
      <OpsResultView canisterId={canisterId} databaseId={databaseId} readMode={readMode} result={result} />
      {activeAction.kind === "recent" ? <RecentPanel canisterId={canisterId} databaseId={databaseId} readIdentity={readIdentity} readMode={readMode} /> : null}
    </div>
  );
}

function OpsButton({ icon, label, onClick }: { icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <button className="inline-flex items-center justify-center gap-1.5 rounded-lg border border-line bg-white px-2 py-2 text-xs text-ink hover:border-accent" type="button" onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

function ActionPreview({ action, busy, onConfirm }: { action: OpsAction; busy: boolean; onConfirm: (() => void) | null }) {
  return (
    <div className="border-b border-line bg-white px-3 py-2 text-xs">
      <div className="grid gap-1 font-mono text-muted">
        <span>intent: {action.kind}</span>
        <span>target: {action.targetPath ?? "current database"}</span>
        <span>identity: {action.identityMode}</span>
        <span>side effect: {action.sideEffect}</span>
      </div>
      {onConfirm ? (
        <button className="mt-2 inline-flex items-center gap-1.5 rounded-lg border border-accent bg-accent px-3 py-1.5 text-xs font-medium text-white disabled:opacity-60" disabled={busy} type="button" onClick={onConfirm}>
          <Link2 size={13} />
          {busy ? "Queueing..." : "Confirm queue"}
        </button>
      ) : null}
    </div>
  );
}

function OpsResultView({ canisterId, databaseId, readMode, result }: { canisterId: string; databaseId: string; readMode: "anonymous" | null; result: OpsResult | null }) {
  if (!result) return null;
  if (result.kind === "message") {
    return <div className={`m-3 rounded-lg border px-3 py-2 text-xs ${result.tone === "error" ? "border-red-200 bg-red-50 text-red-900" : "border-line bg-white text-ink"}`}>{result.text}</div>;
  }
  if (result.kind === "answer") {
    return (
      <div className={`m-3 space-y-2 rounded-lg border bg-white p-3 text-xs ${result.abstained ? "border-yellow-200" : "border-line"}`}>
        <p className="whitespace-pre-wrap text-ink">{result.answer}</p>
        {result.abstained ? <p className="text-yellow-800">根拠不足を含む回答。</p> : null}
        {result.citations.length > 0 ? (
          <div className="space-y-1">
            <p className="font-semibold text-muted">Citations</p>
            {result.citations.map((path) => (
              <a key={path} className="block truncate font-mono text-accent no-underline hover:underline" href={hrefForPath(canisterId, databaseId, path, undefined, "ops", undefined, undefined, readMode)}>
                {path}
              </a>
            ))}
          </div>
        ) : null}
      </div>
    );
  }
  return (
    <div className="m-3 space-y-2 rounded-lg border border-line bg-white p-3 text-xs">
      <a className="font-mono text-accent no-underline hover:underline" href={hrefForPath(canisterId, databaseId, result.targetPath, "raw", "ops", undefined, undefined, readMode)}>
        {result.targetPath}
      </a>
      {result.hints.length === 0 ? <p className="text-green-700">No lightweight warnings.</p> : null}
      {result.hints.map((hint) => (
        <div key={`${hint.title}:${hint.line ?? "note"}`} className="rounded border border-yellow-200 bg-yellow-50 p-2 text-yellow-950">
          <p className="font-semibold">{hint.title}</p>
          <p className="mt-1">{hint.detail}</p>
          {hint.preview ? <p className="mt-1 font-mono">{hint.preview}</p> : null}
        </div>
      ))}
    </div>
  );
}

function isAnswerBody(value: unknown): value is { answer: string; citations: string[]; abstained: boolean } {
  return isRecord(value) && typeof value.answer === "string" && Array.isArray(value.citations) && value.citations.every((item) => typeof item === "string") && typeof value.abstained === "boolean";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
