"use client";

import { AuthClient } from "@icp-sdk/auth/client";
import type { Identity } from "@icp-sdk/core/agent";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { PackageManager, RoleBanner } from "@/app/skills/skill-registry-management-ui";
import { usePackageManager } from "@/app/skills/skill-registry-package-state";
import { EmptyState, SkillCard, StatusPanel, SummaryStrip } from "@/app/skills/skill-registry-ui";
import { DELEGATION_TTL_NS, identityProviderUrl } from "@/lib/auth";
import { filterSkills, loadSkillCatalog, summarizeSkills, type CatalogSkill, type StatusFilter } from "@/lib/skill-registry-catalog";
import { applyProposalDiff, previewApplyProposalDiff, type ProposalDiffPreview } from "@/lib/skill-registry-diff";
import { approveSkillProposal, recordSkillEvent, recordSkillRun, updateSkillStatus, type RunOutcome, type SkillStatus } from "@/lib/skill-registry-operations";
import type { DatabaseRole } from "@/lib/types";
import { listDatabasesAuthenticated } from "@/lib/vfs-client";

type LoadState = "idle" | "loading" | "ready" | "error";
type ActionDraft = {
  busy: boolean;
  error: string | null;
  message: string | null;
  preview: ProposalDiffPreview | null;
  statusReason: string;
  runTask: string;
  runOutcome: RunOutcome;
  runAgent: string;
  runNotes: string;
};

const DEFAULT_ACTION: ActionDraft = {
  busy: false,
  error: null,
  message: null,
  preview: null,
  statusReason: "",
  runTask: "",
  runOutcome: "success",
  runAgent: "browser",
  runNotes: ""
};

export function SkillRegistryClient({ databaseId }: { databaseId: string }) {
  const canisterId = process.env.NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID ?? "";
  const refreshSeqRef = useRef(0);
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [principal, setPrincipal] = useState<string | null>(null);
  const [skills, setSkills] = useState<CatalogSkill[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("active");
  const [actions, setActions] = useState<Record<string, ActionDraft>>({});
  const [databaseRole, setDatabaseRole] = useState<DatabaseRole | null>(null);

  const loadCatalog = useCallback(
    async (identity?: Identity) => {
      const refreshSeq = (refreshSeqRef.current += 1);
      const isCurrentRefresh = () => refreshSeq === refreshSeqRef.current;
      if (!canisterId) {
        setError("NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID is not configured.");
        setLoadState("error");
        return;
      }
      if (!databaseId) {
        setError("Database id is missing.");
        setLoadState("error");
        return;
      }
      setLoadState("loading");
      setError(null);
      try {
        const nextSkills = await loadSkillCatalog(canisterId, databaseId, identity);
        if (!isCurrentRefresh()) return;
        setSkills(nextSkills);
        setLoadState("ready");
      } catch (cause) {
        if (!isCurrentRefresh()) return;
        setError(errorMessage(cause));
        setLoadState("error");
      }
    },
    [canisterId, databaseId]
  );

  const loadRole = useCallback(async (activeIdentity: Identity) => {
    try {
      const databases = await listDatabasesAuthenticated(canisterId, activeIdentity);
      setDatabaseRole(databases.find((database) => database.databaseId === databaseId)?.role ?? null);
    } catch {
      setDatabaseRole(null);
    }
  }, [canisterId, databaseId]);

  useEffect(() => {
    let cancelled = false;
    AuthClient.create()
      .then(async (client) => {
        if (cancelled) return;
        setAuthClient(client);
        if (await client.isAuthenticated()) {
          const identity = client.getIdentity();
          setPrincipal(identity.getPrincipal().toText());
          await loadRole(identity);
          await loadCatalog(identity);
        } else {
          await loadCatalog();
        }
      })
      .catch((cause) => {
        if (cancelled) return;
        setError(errorMessage(cause));
        setLoadState("error");
      });
    return () => {
      cancelled = true;
    };
  }, [loadCatalog, loadRole]);

  async function login() {
    if (!authClient) return;
    setError(null);
    await authClient.login({
      identityProvider: identityProviderUrl(),
      maxTimeToLive: DELEGATION_TTL_NS,
      onSuccess: () => {
        const identity = authClient.getIdentity();
        setPrincipal(identity.getPrincipal().toText());
        void loadRole(identity);
        void loadCatalog(identity);
      },
      onError: (cause) => {
        setError(errorMessage(cause));
        setLoadState("error");
      }
    });
  }

  async function logout() {
    if (!authClient) return;
    refreshSeqRef.current += 1;
    await authClient.logout();
    setPrincipal(null);
    setDatabaseRole(null);
    setSkills([]);
    setError(null);
    setLoadState("idle");
    await loadCatalog();
  }

  const filteredSkills = useMemo(() => filterSkills(skills, query, statusFilter), [skills, query, statusFilter]);
  const summary = useMemo(() => summarizeSkills(skills), [skills]);
  const identity = authClient?.getIdentity();
  const writable = databaseRole === "writer" || databaseRole === "owner";
  const packageManager = usePackageManager({ canisterId, databaseId, identity, writable, refresh: loadCatalog, errorMessage });

  function actionFor(skill: CatalogSkill): ActionDraft {
    return actions[skill.manifest.id] ?? DEFAULT_ACTION;
  }

  function patchAction(skill: CatalogSkill, patch: Partial<ActionDraft>) {
    setActions((current) => ({ ...current, [skill.manifest.id]: { ...DEFAULT_ACTION, ...current[skill.manifest.id], ...patch } }));
  }

  async function runSkillAction(skill: CatalogSkill, operation: (identity: Identity, draft: ActionDraft) => Promise<void>, clearRun = false) {
    if (!identity) {
      patchAction(skill, { error: "Login is required." });
      return;
    }
    const draft = actionFor(skill);
    patchAction(skill, { busy: true, error: null });
    try {
      await operation(identity, draft);
      patchAction(skill, clearRun ? { busy: false, runTask: "", runNotes: "", message: "Operation completed." } : { busy: false, message: "Operation completed." });
      await loadCatalog(identity);
    } catch (cause) {
      patchAction(skill, { busy: false, error: errorMessage(cause) });
    }
  }

  return (
    <main className="min-h-screen px-6 py-8">
      <section className="mx-auto flex max-w-7xl flex-col gap-6">
        <header className="flex flex-col gap-4 border-b border-line pb-5 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <div className="flex flex-wrap items-center gap-3 text-sm">
              <Link className="text-accent no-underline hover:underline" href="/">
                Dashboard
              </Link>
              <Link className="text-accent no-underline hover:underline" href={`/${encodeURIComponent(databaseId)}/Wiki`}>
                Wiki
              </Link>
            </div>
            <h1 className="mt-2 text-3xl font-semibold text-ink">Skill Registry</h1>
            <p className="mt-1 font-mono text-xs text-muted">{databaseId || "unknown database"}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {principal ? <span className="max-w-[320px] truncate rounded-lg border border-line bg-white px-3 py-2 font-mono text-xs text-muted">{principal}</span> : null}
            <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink disabled:opacity-60" disabled={loadState === "loading"} type="button" onClick={() => void loadCatalog(authClient?.getIdentity())}>
              <RefreshCw aria-hidden size={15} />
              <span className="ml-2">Refresh</span>
            </button>
            {principal ? (
              <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink" type="button" onClick={() => void logout()}>
                Logout
              </button>
            ) : (
              <button className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white disabled:opacity-60" disabled={!authClient} type="button" onClick={() => void login()}>
                Login
              </button>
            )}
          </div>
        </header>

        <SummaryStrip summary={summary} />
        <RoleBanner role={databaseRole} principal={principal} />
        <PackageManager draft={packageManager.draft} busy={packageManager.busy} writable={writable} message={packageManager.message} handlers={packageManager.handlers} />

        <section className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
          <label className="flex items-center gap-2 rounded-lg border border-line bg-paper px-3 py-2">
            <Search aria-hidden className="text-muted" size={17} />
            <input
              className="min-w-0 flex-1 bg-transparent text-sm text-ink outline-none placeholder:text-muted"
              placeholder="Search skills, tags, use cases, provenance"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <div className="inline-flex overflow-hidden rounded-lg border border-line bg-white text-sm">
            {(["active", "all", "deprecated"] as const).map((value) => (
              <button key={value} className={`px-3 py-2 capitalize ${statusFilter === value ? "bg-accent text-white" : "text-ink"}`} type="button" onClick={() => setStatusFilter(value)}>
                {value}
              </button>
            ))}
          </div>
        </section>

        {error ? <StatusPanel tone="error" message={error} /> : null}
        {loadState === "loading" ? <StatusPanel tone="info" message="Loading skill registry..." /> : null}
        {loadState === "ready" && skills.length === 0 ? <EmptyState databaseId={databaseId} /> : null}

        {filteredSkills.length > 0 ? (
          <section className="grid gap-3 lg:grid-cols-2">
            {filteredSkills.map((skill) => (
              <SkillCard
                key={skill.manifestPath}
                canisterId={canisterId}
                databaseId={databaseId}
                skill={skill}
                authenticated={Boolean(principal)}
                writable={writable}
                action={actionFor(skill)}
                handlers={{
                  setStatusReason: (value) => patchAction(skill, { statusReason: value }),
                  setRunTask: (value) => patchAction(skill, { runTask: value }),
                  setRunOutcome: (value) => patchAction(skill, { runOutcome: value }),
                  setRunAgent: (value) => patchAction(skill, { runAgent: value }),
                  setRunNotes: (value) => patchAction(skill, { runNotes: value }),
                  updateStatus: (status: SkillStatus) => void runSkillAction(skill, (activeIdentity, draft) => updateSkillStatus(canisterId, databaseId, activeIdentity, skill, status, draft.statusReason)),
                  recordRun: () =>
                    void runSkillAction(
                      skill,
                      (activeIdentity, draft) =>
                        recordSkillRun(canisterId, databaseId, activeIdentity, skill, {
                          task: draft.runTask,
                          outcome: draft.runOutcome,
                          agent: draft.runAgent,
                          notes: draft.runNotes
                        }),
                      true
                    ),
                  approveProposal: (proposal) => void runSkillAction(skill, (activeIdentity) => approveSkillProposal(canisterId, databaseId, activeIdentity, skill, proposal.path)),
                  previewProposal: (proposal) =>
                    void runSkillAction(skill, async (activeIdentity) => {
                      const preview = await previewApplyProposalDiff(canisterId, databaseId, activeIdentity, skill, proposal);
                      patchAction(skill, { preview, message: `Preview ready: ${preview.targetPath}` });
                    }),
                  applyProposal: (proposal) =>
                    void runSkillAction(skill, async (activeIdentity, draft) => {
                      if (!draft.preview || draft.preview.proposalPath !== proposal.path) throw new Error("Preview this proposal before applying.");
                      await applyProposalDiff(canisterId, databaseId, activeIdentity, proposal, draft.preview);
                      await recordSkillEvent(canisterId, databaseId, activeIdentity, skill.manifest.id, { action: "proposal.apply", targetPath: draft.preview.targetPath, result: "applied" });
                    })
                }}
              />
            ))}
          </section>
        ) : loadState === "ready" && skills.length > 0 ? (
          <StatusPanel tone="info" message="No skills match the current filter." />
        ) : null}
      </section>
    </main>
  );
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : "Unexpected error";
}
