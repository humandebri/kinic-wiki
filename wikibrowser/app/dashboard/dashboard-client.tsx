"use client";

import { AuthClient } from "@icp-sdk/auth/client";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AuthControls, OwnerPanel, StatusPanel, SummaryPanel } from "./dashboard-ui";
import { DELEGATION_TTL_NS, identityProviderUrl } from "@/lib/auth";
import type { DatabaseMember, DatabaseRole, DatabaseSummary } from "@/lib/types";
import {
  grantDatabaseAccessAuthenticated,
  listDatabaseMembersAuthenticated,
  listDatabasesAuthenticated,
  revokeDatabaseAccessAuthenticated
} from "@/lib/vfs-client";

type LoadState = "idle" | "loading" | "ready" | "error";

export function DashboardDatabaseClient({ databaseId }: { databaseId: string }) {
  const canisterId = process.env.NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID ?? "";
  const refreshSeqRef = useRef(0);
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [principal, setPrincipal] = useState<string | null>(null);
  const [databases, setDatabases] = useState<DatabaseSummary[]>([]);
  const [members, setMembers] = useState<DatabaseMember[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [memberError, setMemberError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [actionTone, setActionTone] = useState<"error" | "info">("info");
  const [busy, setBusy] = useState(false);

  const database = useMemo(() => databases.find((item) => item.databaseId === databaseId) ?? null, [databaseId, databases]);
  const canManage = database?.role === "owner" && !memberError;

  const refresh = useCallback(
    async (client: AuthClient, nextDatabaseId: string) => {
      const refreshSeq = (refreshSeqRef.current += 1);
      const isCurrentRefresh = () => refreshSeq === refreshSeqRef.current;
      if (!canisterId) {
        setError("NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID is not configured.");
        setLoadState("error");
        return;
      }
      if (!nextDatabaseId) {
        setError("Database id is missing.");
        setLoadState("error");
        return;
      }
      setLoadState("loading");
      setError(null);
      setMemberError(null);
      try {
        const identity = client.getIdentity();
        const nextDatabases = await listDatabasesAuthenticated(canisterId, identity);
        if (!isCurrentRefresh()) return;
        const nextDatabase = nextDatabases.find((item) => item.databaseId === nextDatabaseId) ?? null;
        setPrincipal(identity.getPrincipal().toText());
        setDatabases(nextDatabases);
        setMembers([]);
        if (nextDatabase?.role === "owner") {
          try {
            const nextMembers = await listDatabaseMembersAuthenticated(canisterId, identity, nextDatabaseId);
            if (!isCurrentRefresh()) return;
            setMembers(nextMembers);
          } catch (cause) {
            if (!isCurrentRefresh()) return;
            setMemberError(errorMessage(cause));
          }
        }
        if (!isCurrentRefresh()) return;
        setLoadState("ready");
      } catch (cause) {
        if (!isCurrentRefresh()) return;
        setError(errorMessage(cause));
        setLoadState("error");
      }
    },
    [canisterId]
  );

  useEffect(() => {
    let cancelled = false;
    AuthClient.create()
      .then(async (client) => {
        if (cancelled) return;
        setAuthClient(client);
        if (await client.isAuthenticated()) {
          await refresh(client, databaseId);
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
  }, [databaseId, refresh]);

  async function login() {
    if (!authClient) return;
    setError(null);
    await authClient.login({
      identityProvider: identityProviderUrl(),
      maxTimeToLive: DELEGATION_TTL_NS,
      onSuccess: () => {
        void refresh(authClient, databaseId);
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
    setDatabases([]);
    setMembers([]);
    setError(null);
    setMemberError(null);
    setLoadState("idle");
  }

  async function grantAccess(principalText: string, role: DatabaseRole) {
    if (!authClient || !databaseId) return;
    setBusy(true);
    setActionMessage(null);
    try {
      await grantDatabaseAccessAuthenticated(canisterId, authClient.getIdentity(), databaseId, principalText, role);
      setActionTone("info");
      setActionMessage("Access granted.");
      await refresh(authClient, databaseId);
    } catch (cause) {
      setActionTone("error");
      setActionMessage(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function revokeAccess(principalText: string) {
    if (!authClient || !databaseId) return;
    setBusy(true);
    setActionMessage(null);
    try {
      await revokeDatabaseAccessAuthenticated(canisterId, authClient.getIdentity(), databaseId, principalText);
      setActionTone("info");
      setActionMessage("Access revoked.");
      await refresh(authClient, databaseId);
    } catch (cause) {
      setActionTone("error");
      setActionMessage(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="min-h-screen px-6 py-8">
      <section className="mx-auto flex max-w-6xl flex-col gap-6">
        <header className="flex flex-col gap-4 border-b border-line pb-5 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <Link className="text-sm text-accent no-underline hover:underline" href="/">
              Dashboard
            </Link>
            <h1 className="mt-2 text-3xl font-semibold text-ink">Database access</h1>
            <p className="mt-1 font-mono text-xs text-muted">{databaseId || "unknown database"}</p>
          </div>
          <AuthControls authReady={Boolean(authClient)} loading={loadState === "loading"} principal={principal} onLogin={login} onLogout={logout} />
        </header>

        {error ? <StatusPanel tone="error" message={error} /> : null}
        {actionMessage ? <StatusPanel tone={actionTone} message={actionMessage} /> : null}

        {principal ? (
          <>
            <SummaryPanel database={database} databaseId={databaseId} principal={principal} />
            {database ? (
              canManage ? (
                <OwnerPanel busy={busy} members={members} principal={principal} onGrant={grantAccess} onRevoke={revokeAccess} />
              ) : (
                <StatusPanel tone="info" message={memberError ?? "No management permission for this database."} />
              )
            ) : (
              <StatusPanel tone="error" message="Database is not linked to this principal." />
            )}
          </>
        ) : (
          <section className="rounded-lg border border-line bg-paper p-8 shadow-sm">
            <p className="text-sm leading-6 text-muted">Login with Internet Identity to manage database access.</p>
          </section>
        )}
      </section>
    </main>
  );
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : "Unexpected error";
}
