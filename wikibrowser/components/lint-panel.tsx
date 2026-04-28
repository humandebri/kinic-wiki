"use client";

import { AlertTriangle } from "lucide-react";
import { collectLintHints } from "@/lib/lint-hints";
import type { WikiNode } from "@/lib/types";

export function LintPanel({ path, node }: { path: string; node: WikiNode | null }) {
  if (!node) {
    return <p className="p-4 text-sm text-muted">Select a markdown node to inspect lightweight hints.</p>;
  }
  const hints = collectLintHints(path, node.content);
  return (
    <div className="space-y-2 overflow-auto p-3">
      {hints.length === 0 ? (
        <div className="rounded-xl border border-green-200 bg-green-50 p-3 text-sm text-green-800">
          No lightweight warnings for this note.
        </div>
      ) : null}
      {hints.map((hint) => (
        <div key={`${hint.title}-${hint.line}`} className="rounded-xl border border-yellow-200 bg-yellow-50 p-3 text-sm">
          <div className="flex items-center gap-2 text-yellow-800">
            <AlertTriangle size={14} />
            <span className="font-semibold">{hint.title}</span>
          </div>
          <p className="mt-2 text-xs text-yellow-900">{hint.detail}</p>
          {hint.line ? <p className="mt-2 font-mono text-[11px] text-yellow-700">line {hint.line}</p> : null}
        </div>
      ))}
    </div>
  );
}
