"use client";

import { AlertTriangle, GitBranch, Info, Sparkles } from "lucide-react";
import { collectLintHints } from "@/lib/lint-hints";
import type { ChildNode, WikiNode } from "@/lib/types";
import { InspectorCard, Meta } from "@/components/panel";

export function Inspector({
  path,
  node,
  childNodes,
  noteRole,
  outgoingLinks
}: {
  path: string;
  node: WikiNode | null;
  childNodes: ChildNode[];
  noteRole: string;
  outgoingLinks: string[];
}) {
  const kind = node?.kind ?? "directory";
  const size = node ? `${new TextEncoder().encode(node.content).length}` : null;
  const hints = node ? collectLintHints(path, node.content) : [];
  return (
    <div className="space-y-4 overflow-auto p-4 text-sm">
      <InspectorCard title="Identity" icon={<Info size={15} />}>
        <Meta label="path" value={path} />
        <Meta label="kind" value={kind} />
        <Meta label="role" value={noteRole} />
        {node ? <Meta label="size_bytes" value={size} /> : <Meta label="children" value={String(childNodes.length)} />}
      </InspectorCard>
      <InspectorCard title="Metadata" icon={<Sparkles size={15} />}>
        <Meta label="updated_at" value={node?.updatedAt ?? "virtual"} />
        <Meta label="etag" value={node?.etag ?? "virtual"} />
      </InspectorCard>
      <InspectorCard title="Lint Hints" icon={<AlertTriangle size={15} />}>
        {hints.length > 0 ? (
          <ul className="space-y-2">
            {hints.slice(0, 5).map((hint) => (
              <li key={`${hint.title}-${hint.line}`} className="rounded-lg border border-yellow-200 bg-yellow-50 p-2">
                <p className="text-xs font-semibold text-yellow-800">{hint.title}</p>
                <p className="mt-1 text-xs text-yellow-900">{hint.detail}</p>
                {hint.line ? <p className="mt-1 font-mono text-[11px] text-yellow-700">line {hint.line}</p> : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted">No lightweight warnings.</p>
        )}
      </InspectorCard>
      <InspectorCard title="Outgoing Links" icon={<GitBranch size={15} />}>
        {outgoingLinks.length > 0 ? (
          <ul className="space-y-1">
            {outgoingLinks.map((link) => (
              <li key={link} className="truncate font-mono text-xs text-muted">
                {link}
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted">No markdown links detected.</p>
        )}
      </InspectorCard>
    </div>
  );
}
