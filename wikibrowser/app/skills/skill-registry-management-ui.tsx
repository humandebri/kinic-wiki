"use client";

import { Github, Upload } from "lucide-react";
import type { DatabaseRole } from "@/lib/types";
import type { SkillCatalog } from "@/lib/skill-registry-package";

export type PackageDraft = {
  source: string;
  reference: string;
  id: string;
  catalog: SkillCatalog;
  skill: string;
  manifest: string;
  provenance: string;
  evals: string;
  extraName: string;
  extraContent: string;
};

export type PackageHandlers = {
  setDraft: (patch: Partial<PackageDraft>) => void;
  importGitHub: () => void;
  pasteUpsert: () => void;
};

export function RoleBanner({ role, principal }: { role: DatabaseRole | null; principal: string | null }) {
  const writable = role === "writer" || role === "owner";
  return (
    <section className="rounded-lg border border-line bg-paper p-4 text-sm">
      <p className="font-medium text-ink">Database role: {role ?? (principal ? "unknown" : "anonymous")}</p>
      <p className="mt-1 text-muted">{writable ? "Write operations are enabled for this database." : "Write operations require writer or owner access."}</p>
    </section>
  );
}

export function PackageManager({
  draft,
  busy,
  writable,
  message,
  handlers
}: {
  draft: PackageDraft;
  busy: boolean;
  writable: boolean;
  message: string | null;
  handlers: PackageHandlers;
}) {
  return (
    <section className="grid gap-4 rounded-lg border border-line bg-paper p-4 lg:grid-cols-2">
      <div>
        <div className="flex items-center gap-2 text-sm font-medium text-ink">
          <Github aria-hidden size={16} />
          GitHub import/update
        </div>
        <div className="mt-3 grid gap-2">
          <input className="rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="owner/repo:path" value={draft.source} onChange={(event) => handlers.setDraft({ source: event.target.value })} />
          <div className="grid gap-2 sm:grid-cols-3">
            <input className="rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="ref" value={draft.reference} onChange={(event) => handlers.setDraft({ reference: event.target.value })} />
            <input className="rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="skill id" value={draft.id} onChange={(event) => handlers.setDraft({ id: event.target.value })} />
            <CatalogSelect value={draft.catalog} onChange={(catalog) => handlers.setDraft({ catalog })} />
          </div>
          <p className="text-xs text-muted">Public GitHub repositories only. Prune is unavailable because the browser API has no delete operation.</p>
          <button className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white disabled:opacity-50" disabled={!writable || busy || !draft.source.trim() || !draft.id.trim()} type="button" onClick={handlers.importGitHub}>
            Import from GitHub
          </button>
        </div>
      </div>
      <div>
        <div className="flex items-center gap-2 text-sm font-medium text-ink">
          <Upload aria-hidden size={16} />
          Paste upsert
        </div>
        <div className="mt-3 grid gap-2">
          <textarea className="min-h-24 rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="SKILL.md" value={draft.skill} onChange={(event) => handlers.setDraft({ skill: event.target.value })} />
          <textarea className="min-h-16 rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="manifest.md (optional)" value={draft.manifest} onChange={(event) => handlers.setDraft({ manifest: event.target.value })} />
          <div className="grid gap-2 sm:grid-cols-3">
            <input className="rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="extra file.md" value={draft.extraName} onChange={(event) => handlers.setDraft({ extraName: event.target.value })} />
            <input className="rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="skill id" value={draft.id} onChange={(event) => handlers.setDraft({ id: event.target.value })} />
            <CatalogSelect value={draft.catalog} onChange={(catalog) => handlers.setDraft({ catalog })} />
          </div>
          <textarea className="min-h-16 rounded-lg border border-line bg-white px-3 py-2 text-sm" placeholder="extra markdown content" value={draft.extraContent} onChange={(event) => handlers.setDraft({ extraContent: event.target.value })} />
          <button className="rounded-lg border border-accent bg-accent px-3 py-2 text-sm font-medium text-white disabled:opacity-50" disabled={!writable || busy || !draft.skill.trim() || !draft.id.trim()} type="button" onClick={handlers.pasteUpsert}>
            Upsert pasted package
          </button>
        </div>
      </div>
      {message ? <p className="lg:col-span-2 rounded-lg border border-line bg-white px-3 py-2 text-xs text-muted">{message}</p> : null}
    </section>
  );
}

function CatalogSelect({ value, onChange }: { value: SkillCatalog; onChange: (value: SkillCatalog) => void }) {
  return (
    <select className="rounded-lg border border-line bg-white px-3 py-2 text-sm" value={value} onChange={(event) => onChange(event.target.value === "public" ? "public" : "private")}>
      <option value="private">private</option>
      <option value="public">public</option>
    </select>
  );
}
