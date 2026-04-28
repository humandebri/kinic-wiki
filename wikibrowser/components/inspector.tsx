"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { AlertTriangle, GitBranch, Info, Sparkles } from "lucide-react";
import { collectLintHints, provenancePathFor, rawSourceLinksFor } from "@/lib/lint-hints";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode, WikiNode } from "@/lib/types";
import { apiPath, fetchJson } from "@/lib/wiki-helpers";
import { InspectorCard, Meta } from "@/components/panel";

type ProvenanceState = {
  path: string | null;
  links: string[];
};

export function Inspector({
  canisterId,
  path,
  node,
  childNodes,
  noteRole,
  outgoingLinks
}: {
  canisterId: string;
  path: string;
  node: WikiNode | null;
  childNodes: ChildNode[];
  noteRole: string;
  outgoingLinks: string[];
}) {
  const kind = node?.kind ?? "directory";
  const size = node ? `${new TextEncoder().encode(node.content).length}` : null;
  const hints = node ? collectLintHints(path, node.content) : [];
  const directRawSourceLinks = node ? rawSourceLinksFor(path, node.content) : [];
  const expectedProvenancePath = node && directRawSourceLinks.length === 0 ? provenancePathFor(path) : null;
  const [provenance, setProvenance] = useState<ProvenanceState>({ path: null, links: [] });
  const inferredRawSourceLinks = provenance.path === expectedProvenancePath ? provenance.links : [];
  const rawSourceLinks = directRawSourceLinks.length > 0 ? directRawSourceLinks : inferredRawSourceLinks;
  const loadingRawSource = Boolean(expectedProvenancePath && provenance.path !== expectedProvenancePath);

  useEffect(() => {
    if (!expectedProvenancePath) {
      return;
    }
    let cancelled = false;
    fetchJson<WikiNode>(apiPath(canisterId, "node", new URLSearchParams({ path: expectedProvenancePath })))
      .then((provenanceNode) => {
        if (!cancelled) {
          setProvenance({
            path: expectedProvenancePath,
            links: rawSourceLinksFor(expectedProvenancePath, provenanceNode.content)
          });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setProvenance({ path: expectedProvenancePath, links: [] });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId, expectedProvenancePath]);

  return (
    <div className="min-h-0 flex-1 space-y-4 overflow-auto p-4 text-sm">
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
                {hint.preview ? <p className="mt-1 rounded bg-white/70 p-2 font-mono text-[11px] text-yellow-950">{hint.preview}</p> : null}
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
              <li key={link} className="truncate font-mono text-xs">
                <Link className="text-accent no-underline hover:underline" href={linkHref(canisterId, link)}>
                  {link}
                </Link>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted">No markdown links detected.</p>
        )}
      </InspectorCard>
      <InspectorCard title="Raw Source" icon={<GitBranch size={15} />}>
        {rawSourceLinks.length > 0 ? (
          <ul className="space-y-1">
            {rawSourceLinks.map((link) => (
              <li key={link} className="truncate font-mono text-xs">
                <Link className="text-accent no-underline hover:underline" href={hrefForPath(canisterId, link)}>
                  {link}
                </Link>
              </li>
            ))}
          </ul>
        ) : loadingRawSource ? (
          <p className="text-xs text-muted">Checking provenance...</p>
        ) : (
          <p className="text-xs text-muted">No raw source path inferred.</p>
        )}
      </InspectorCard>
    </div>
  );
}

function linkHref(canisterId: string, link: string): string {
  if (link.startsWith("/Wiki") || link.startsWith("/Sources")) {
    return hrefForPath(canisterId, link);
  }
  if (link.startsWith("http://") || link.startsWith("https://")) {
    return link;
  }
  return hrefForPath(canisterId, `/Wiki/${link.replace(/^\/+/, "")}`);
}
