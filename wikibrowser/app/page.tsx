"use client";

import { AuthClient } from "@icp-sdk/auth/client";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import type { DatabaseSummary } from "@/lib/types";
import { createDatabaseAuthenticated, listDatabasesAuthenticated } from "@/lib/vfs-client";

const DELEGATION_TTL_NS = BigInt(8) * BigInt(3_600_000_000_000);

type LoadState = "idle" | "loading" | "ready" | "error";

export default function HomePage() {
  const canisterId = process.env.KINIC_WIKI_CANISTER_ID ?? "";
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [principal, setPrincipal] = useState<string | null>(null);
  const [databases, setDatabases] = useState<DatabaseSummary[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [createdDatabaseId, setCreatedDatabaseId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const refreshDatabases = useCallback(
    async (client: AuthClient) => {
      if (!canisterId) {
        setError("KINIC_WIKI_CANISTER_ID is not configured.");
        setLoadState("error");
        return;
      }
      setLoadState("loading");
      setError(null);
      try {
        const identity = client.getIdentity();
        const nextDatabases = await listDatabasesAuthenticated(canisterId, identity);
        setDatabases(nextDatabases);
        setPrincipal(identity.getPrincipal().toText());
        setLoadState("ready");
      } catch (cause) {
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
          await refreshDatabases(client);
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
  }, [refreshDatabases]);

  async function login() {
    if (!authClient) return;
    setError(null);
    await authClient.login({
      identityProvider: identityProviderUrl(),
      maxTimeToLive: DELEGATION_TTL_NS,
      onSuccess: () => {
        void refreshDatabases(authClient);
      },
      onError: (cause) => {
        setError(errorMessage(cause));
        setLoadState("error");
      }
    });
  }

  async function logout() {
    if (!authClient) return;
    await authClient.logout();
    setPrincipal(null);
    setDatabases([]);
    setCreatedDatabaseId(null);
    setError(null);
    setLoadState("idle");
  }

  async function createDatabase() {
    if (!authClient || !canisterId) return;
    setCreating(true);
    setError(null);
    try {
      const databaseId = await createDatabaseAuthenticated(canisterId, authClient.getIdentity());
      setCreatedDatabaseId(databaseId);
      await refreshDatabases(authClient);
    } catch (cause) {
      setError(errorMessage(cause));
      setLoadState("error");
    } finally {
      setCreating(false);
    }
  }

  return (
    <main className="min-h-screen px-6 py-8">
      <section className="mx-auto flex max-w-6xl flex-col gap-6">
        <header className="flex flex-col gap-4 border-b border-line pb-5 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Kinic Wiki</p>
            <h1 className="mt-2 text-3xl font-semibold text-ink">Database dashboard</h1>
          </div>
          <AuthControls
            authReady={Boolean(authClient)}
            principal={principal}
            loading={loadState === "loading"}
            onLogin={login}
            onLogout={logout}
            onRefresh={() => {
              if (authClient) void refreshDatabases(authClient);
            }}
          />
        </header>

        {error ? <StatusPanel tone="error" message={error} /> : null}
        {createdDatabaseId ? <CreatedDatabasePanel databaseId={createdDatabaseId} /> : null}

        {principal ? (
          <section className="rounded-lg border border-line bg-paper shadow-sm">
            <div className="flex flex-col gap-3 border-b border-line px-4 py-4 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <h2 className="text-lg font-semibold text-ink">Databases</h2>
                <p className="mt-1 font-mono text-xs text-muted">{principal}</p>
              </div>
              <button
                className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
                disabled={creating || loadState === "loading"}
                type="button"
                onClick={createDatabase}
              >
                {creating ? "Creating..." : "Create database"}
              </button>
            </div>
            <DatabaseBody databases={databases} loading={loadState === "loading"} />
          </section>
        ) : (
          <section className="rounded-lg border border-line bg-paper p-8 shadow-sm">
            <p className="text-sm leading-6 text-muted">
              Login with Internet Identity to list databases where your principal has membership.
            </p>
          </section>
        )}
      </section>
    </main>
  );
}

function AuthControls({
  authReady,
  principal,
  loading,
  onLogin,
  onLogout,
  onRefresh
}: {
  authReady: boolean;
  principal: string | null;
  loading: boolean;
  onLogin: () => void;
  onLogout: () => void;
  onRefresh: () => void;
}) {
  if (!principal) {
    return (
      <button
        className="rounded-lg border border-accent bg-accent px-4 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
        disabled={!authReady}
        type="button"
        onClick={onLogin}
      >
        Login with Internet Identity
      </button>
    );
  }

  return (
    <div className="flex flex-wrap gap-2">
      <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink" disabled={loading} type="button" onClick={onRefresh}>
        Refresh
      </button>
      <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink" type="button" onClick={onLogout}>
        Logout
      </button>
    </div>
  );
}

function DatabaseBody({ databases, loading }: { databases: DatabaseSummary[]; loading: boolean }) {
  if (loading) {
    return <div className="p-6 text-sm text-muted">Loading databases...</div>;
  }
  if (databases.length === 0) {
    return <div className="p-6 text-sm text-muted">No databases are linked to this principal.</div>;
  }
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-left text-sm">
        <thead className="bg-white/70 text-xs uppercase tracking-[0.12em] text-muted">
          <tr>
            <th className="px-4 py-3 font-medium">Database</th>
            <th className="px-4 py-3 font-medium">Role</th>
            <th className="px-4 py-3 font-medium">Status</th>
            <th className="px-4 py-3 font-medium">Logical size</th>
            <th className="px-4 py-3 font-medium">Archive</th>
            <th className="px-4 py-3 font-medium">Open</th>
            <th className="px-4 py-3 font-medium">Manage</th>
          </tr>
        </thead>
        <tbody>
          {databases.map((database) => (
            <tr key={database.databaseId} className="border-t border-line">
              <td className="px-4 py-3 font-mono text-xs text-ink">{database.databaseId}</td>
              <td className="px-4 py-3 capitalize text-ink">{database.role}</td>
              <td className="px-4 py-3 capitalize text-ink">{database.status}</td>
              <td className="px-4 py-3 text-ink">{formatBytes(database.logicalSizeBytes)}</td>
              <td className="px-4 py-3 text-muted">{databaseMarker(database)}</td>
              <td className="px-4 py-3">
                <Link className="text-accent no-underline hover:underline" href={`/${encodeURIComponent(database.databaseId)}/Wiki`}>
                  Open
                </Link>
              </td>
              <td className="px-4 py-3">
                <Link className="text-accent no-underline hover:underline" href={`/dashboard/${encodeURIComponent(database.databaseId)}`}>
                  Manage
                </Link>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function StatusPanel({ tone, message }: { tone: "error" | "info"; message: string }) {
  const toneClass = tone === "error" ? "border-red-200 bg-red-50 text-red-900" : "border-line bg-paper text-ink";
  return <div className={`rounded-lg border px-4 py-3 text-sm ${toneClass}`}>{message}</div>;
}

function CreatedDatabasePanel({ databaseId }: { databaseId: string }) {
  return (
    <div className="rounded-lg border border-line bg-paper px-4 py-3 text-sm text-ink">
      Created <span className="font-mono">{databaseId}</span>.{" "}
      <Link className="text-accent no-underline hover:underline" href={`/${encodeURIComponent(databaseId)}/Wiki`}>
        Open
      </Link>
    </div>
  );
}

function identityProviderUrl(): string {
  const host = window.location.hostname;
  if (host === "localhost" || host === "127.0.0.1" || host.endsWith(".localhost")) {
    return "http://id.ai.localhost:8000";
  }
  return "https://id.ai";
}

function formatBytes(value: string): string {
  const bytes = Number(value);
  if (!Number.isFinite(bytes)) {
    return value;
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB"];
  let unitIndex = -1;
  let current = bytes;
  while (current >= 1024 && unitIndex < units.length - 1) {
    current /= 1024;
    unitIndex += 1;
  }
  return `${current.toFixed(current >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

function databaseMarker(database: DatabaseSummary): string {
  if (database.deletedAtMs) {
    return `Deleted ${formatTimestamp(database.deletedAtMs)}`;
  }
  if (database.archivedAtMs) {
    return `Archived ${formatTimestamp(database.archivedAtMs)}`;
  }
  return "-";
}

function formatTimestamp(value: string): string {
  const milliseconds = Number(value);
  if (!Number.isFinite(milliseconds)) {
    return value;
  }
  return new Date(milliseconds).toLocaleString();
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : "Unexpected error";
}
