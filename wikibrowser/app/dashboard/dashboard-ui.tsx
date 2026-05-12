"use client";

import Link from "next/link";
import { FormEvent, useState } from "react";
import type { DatabaseMember, DatabaseRole, DatabaseSummary } from "@/lib/types";

const ANONYMOUS_PRINCIPAL = "2vxsx-fae";

type PendingAclAction = {
  title: string;
  message: string;
  confirmLabel: string;
  principalText: string;
  role?: DatabaseRole;
  kind: "grant" | "revoke";
};

export function AuthControls(props: { authReady: boolean; loading: boolean; principal: string | null; onLogin: () => void; onLogout: () => void }) {
  if (!props.principal) {
    return (
      <button className="rounded-lg border border-accent bg-accent px-4 py-2 text-sm font-medium text-white" disabled={!props.authReady} data-tid="login-button" type="button" onClick={props.onLogin}>
        Login with Internet Identity
      </button>
    );
  }
  return (
    <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink" disabled={props.loading} type="button" onClick={props.onLogout}>
      Logout
    </button>
  );
}

export function SummaryPanel({ database, databaseId, principal }: { database: DatabaseSummary | null; databaseId: string; principal: string }) {
  return (
    <section className="grid gap-3 rounded-lg border border-line bg-paper p-4 text-sm shadow-sm sm:grid-cols-2 lg:grid-cols-5">
      <Field label="Principal" value={principal} />
      <Field label="Database" value={databaseId} />
      <Field label="Role" value={database?.role ?? "-"} />
      <Field label="Status" value={database?.status ?? "-"} />
      <Field label="Logical size" value={database ? formatBytes(database.logicalSizeBytes) : "-"} />
      <Link className="text-accent no-underline hover:underline" href={`/${encodeURIComponent(databaseId)}/Wiki`}>
        Open
      </Link>
    </section>
  );
}

export function OwnerPanel(props: {
  busy: boolean;
  members: DatabaseMember[];
  principal: string;
  onGrant: (principalText: string, role: DatabaseRole) => void;
  onRevoke: (principalText: string) => void;
}) {
  const [pendingAction, setPendingAction] = useState<PendingAclAction | null>(null);
  const publicEnabled = props.members.some((member) => member.principal === ANONYMOUS_PRINCIPAL);
  function requestGrant(principalText: string, role: DatabaseRole) {
    if (principalText === ANONYMOUS_PRINCIPAL) {
      setPendingAction({
        title: "Enable public access",
        message: `Grant reader access to anonymous principal ${ANONYMOUS_PRINCIPAL}. Anyone can read this database through the public browser.`,
        confirmLabel: "Enable public",
        principalText,
        role,
        kind: "grant"
      });
      return;
    }
    if (role === "owner") {
      setPendingAction({
        title: "Grant owner access",
        message: `Grant owner access to ${principalText}. Owners can grant and revoke database access.`,
        confirmLabel: "Grant owner",
        principalText,
        role,
        kind: "grant"
      });
      return;
    }
    props.onGrant(principalText, role);
  }
  function requestRevoke(member: DatabaseMember) {
    if (member.principal === ANONYMOUS_PRINCIPAL) {
      setPendingAction({
        title: "Disable public access",
        message: `Revoke anonymous reader access from ${ANONYMOUS_PRINCIPAL}. Public browser reads will stop working for this database.`,
        confirmLabel: "Disable public",
        principalText: member.principal,
        kind: "revoke"
      });
      return;
    }
    if (member.role === "owner") {
      setPendingAction({
        title: "Revoke owner access",
        message: `Revoke owner access from ${member.principal}. This principal will lose database management access.`,
        confirmLabel: "Revoke owner",
        principalText: member.principal,
        kind: "revoke"
      });
      return;
    }
    props.onRevoke(member.principal);
  }
  function confirmPendingAction() {
    if (!pendingAction) return;
    if (pendingAction.kind === "grant" && pendingAction.role) {
      props.onGrant(pendingAction.principalText, pendingAction.role);
    } else {
      props.onRevoke(pendingAction.principalText);
    }
    setPendingAction(null);
  }
  return (
    <section className="rounded-lg border border-line bg-paper shadow-sm">
      <div className="flex flex-col gap-3 border-b border-line px-4 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-lg font-semibold text-ink">Members</h2>
          <p className="mt-1 text-sm text-muted">Public: {publicEnabled ? "enabled" : "disabled"}</p>
        </div>
        {publicEnabled ? (
          <button
            className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink"
            disabled={props.busy}
            type="button"
            onClick={() => {
              const publicMember = props.members.find((member) => member.principal === ANONYMOUS_PRINCIPAL);
              if (publicMember) requestRevoke(publicMember);
            }}
          >
            Disable public
          </button>
        ) : (
          <button className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white" disabled={props.busy} type="button" onClick={() => requestGrant(ANONYMOUS_PRINCIPAL, "reader")}>
            Enable public
          </button>
        )}
      </div>
      <GrantForm busy={props.busy} onGrant={requestGrant} />
      <MemberTable busy={props.busy} members={props.members} principal={props.principal} onRevoke={requestRevoke} />
      {pendingAction ? <ConfirmAclDialog action={pendingAction} busy={props.busy} onCancel={() => setPendingAction(null)} onConfirm={confirmPendingAction} /> : null}
    </section>
  );
}

export function StatusPanel({ tone, message }: { tone: "error" | "info"; message: string }) {
  const toneClass = tone === "error" ? "border-red-200 bg-red-50 text-red-900" : "border-line bg-paper text-ink";
  return <div className={`rounded-lg border px-4 py-3 text-sm ${toneClass}`}>{message}</div>;
}

function GrantForm({ busy, onGrant }: { busy: boolean; onGrant: (principalText: string, role: DatabaseRole) => void }) {
  const [principalText, setPrincipalText] = useState("");
  const [role, setRole] = useState<DatabaseRole>("reader");
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = principalText.trim();
    if (!trimmed) return;
    onGrant(trimmed, role);
    setPrincipalText("");
  }
  const trimmedPrincipal = principalText.trim();
  return (
    <form className="grid gap-3 border-b border-line p-4" onSubmit={submit}>
      <div className="grid gap-3 sm:grid-cols-[1fr_160px_auto]">
        <input className="rounded-lg border border-line px-3 py-2 font-mono text-sm" value={principalText} onChange={(event) => setPrincipalText(event.target.value)} placeholder="principal" />
        <select className="rounded-lg border border-line px-3 py-2 text-sm" value={role} onChange={(event) => setRole(databaseRoleFromValue(event.target.value))}>
          <option value="reader">reader</option>
          <option value="writer">writer</option>
          <option value="owner">owner</option>
        </select>
        <button className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white" disabled={busy} type="submit">
          Grant
        </button>
      </div>
      <p className="text-xs text-muted">{trimmedPrincipal ? `This will grant ${role} access to principal ${trimmedPrincipal}.` : `Enter a principal to grant ${role} access.`}</p>
    </form>
  );
}

function MemberTable(props: { busy: boolean; members: DatabaseMember[]; principal: string; onRevoke: (member: DatabaseMember) => void }) {
  if (props.members.length === 0) {
    return <div className="p-6 text-sm text-muted">No members.</div>;
  }
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-left text-sm">
        <thead className="bg-white/70 text-xs uppercase tracking-[0.12em] text-muted">
          <tr>
            <th className="px-4 py-3 font-medium">Principal</th>
            <th className="px-4 py-3 font-medium">Role</th>
            <th className="px-4 py-3 font-medium">Created</th>
            <th className="px-4 py-3 font-medium">Revoke</th>
          </tr>
        </thead>
        <tbody>
          {props.members.map((member) => (
            <tr key={member.principal} className="border-t border-line">
              <td className="px-4 py-3 font-mono text-xs text-ink">{member.principal}</td>
              <td className="px-4 py-3 text-ink">{member.role}</td>
              <td className="px-4 py-3 text-muted">{formatTimestamp(member.createdAtMs)}</td>
              <td className="px-4 py-3">
                <button className="rounded-lg border border-line bg-white px-3 py-1.5 text-sm text-ink disabled:opacity-50" disabled={props.busy || member.principal === props.principal} type="button" onClick={() => props.onRevoke(member)}>
                  Revoke
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ConfirmAclDialog(props: { action: PendingAclAction; busy: boolean; onCancel: () => void; onConfirm: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/30 px-4">
      <div className="w-full max-w-md rounded-lg border border-line bg-paper p-5 shadow-lg">
        <h3 className="text-lg font-semibold text-ink">{props.action.title}</h3>
        <p className="mt-3 text-sm leading-6 text-muted">{props.action.message}</p>
        <p className="mt-3 break-all rounded-lg border border-line bg-white px-3 py-2 font-mono text-xs text-ink">{props.action.principalText}</p>
        <div className="mt-5 flex justify-end gap-2">
          <button className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink" disabled={props.busy} type="button" onClick={props.onCancel}>
            Cancel
          </button>
          <button className="rounded-lg border border-red-700 bg-red-700 px-3 py-2 text-sm font-medium text-white" disabled={props.busy} type="button" onClick={props.onConfirm}>
            {props.action.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div className="text-xs uppercase tracking-[0.12em] text-muted">{label}</div>
      <div className="mt-1 truncate font-mono text-xs text-ink" title={value}>
        {value}
      </div>
    </div>
  );
}

function databaseRoleFromValue(value: string): DatabaseRole {
  if (value === "owner") return "owner";
  if (value === "writer") return "writer";
  return "reader";
}

function formatBytes(value: string): string {
  const bytes = Number(value);
  if (!Number.isFinite(bytes) || bytes < 1024) return Number.isFinite(bytes) ? `${bytes} B` : value;
  const units = ["KB", "MB", "GB"];
  let current = bytes;
  let unitIndex = -1;
  while (current >= 1024 && unitIndex < units.length - 1) {
    current /= 1024;
    unitIndex += 1;
  }
  return `${current.toFixed(current >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

function formatTimestamp(value: string): string {
  const milliseconds = Number(value);
  return Number.isFinite(milliseconds) ? new Date(milliseconds).toLocaleString() : value;
}
