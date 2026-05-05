"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { AlertTriangle, Copy, GitBranch, Info, PackageCheck, Sparkles } from "lucide-react";
import { Principal } from "@dfinity/principal";
import { collectLintHints, provenancePathFor, rawSourceLinksFor } from "@/lib/lint-hints";
import { hrefForPath } from "@/lib/paths";
import {
  formatSkillAccessCapabilities,
  isAnySkillRegistryPath,
  isPublicSkillRegistryPath,
  manifestPathForSkillRegistryFile,
  parseSkillManifest,
  skillAccessCapabilities,
  skillAccessHint,
  type SkillManifest
} from "@/lib/skill-manifest";
import type { ChildNode, LinkEdge, WikiNode } from "@/lib/types";
import type { PathPolicyEntry } from "@/lib/types";
import { InspectorCard, Meta } from "@/components/panel";

type ProvenanceState = {
  path: string | null;
  links: string[];
};

type SkillManifestState = {
  path: string | null;
  manifest: SkillManifest | null;
};

type SkillAccessState = {
  principal: string;
  authenticated: boolean;
  roles: string[];
  mode: string | null;
};

export function Inspector({
  canisterId,
  path,
  node,
  childNodes,
  noteRole,
  incomingLinks,
  incomingError,
  outgoingLinks
}: {
  canisterId: string;
  path: string;
  node: WikiNode | null;
  childNodes: ChildNode[];
  noteRole: string;
  incomingLinks: LinkEdge[] | null;
  incomingError?: string | null;
  outgoingLinks: LinkEdge[];
}) {
  const kind = node?.kind ?? "directory";
  const size = node ? `${new TextEncoder().encode(node.content).length}` : null;
  const directSkillManifest = node ? parseSkillManifest(node.content) : null;
  const expectedSkillManifestPath = node && !directSkillManifest ? manifestPathForSkillRegistryFile(path) : null;
  const [skillManifestState, setSkillManifestState] = useState<SkillManifestState>({ path: null, manifest: null });
  const [skillAccess, setSkillAccess] = useState<SkillAccessState>({
    principal: "2vxsx-fae",
    authenticated: false,
    roles: [],
    mode: null
  });
  const [policyEntries, setPolicyEntries] = useState<PathPolicyEntry[]>([]);
  const [policyPrincipal, setPolicyPrincipal] = useState("");
  const [policyRole, setPolicyRole] = useState("Reader");
  const [policyError, setPolicyError] = useState<string | null>(null);
  const accessCapabilities = skillAccessCapabilities(skillAccess.roles);
  const isSkillPath = isAnySkillRegistryPath(path);
  const isPublicSkillPath = isPublicSkillRegistryPath(path);
  const policyPath = isPublicSkillPath ? "/Wiki/public-skills" : "/Wiki/skills";
  const accessHint = skillAccessHint(skillAccess.mode, skillAccess.roles, skillAccess.authenticated);
  const skillManifest =
    directSkillManifest ??
    (skillManifestState.path === expectedSkillManifestPath ? skillManifestState.manifest : null);
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
    import("@/lib/vfs-client")
      .then(({ readNode }) => readNode(canisterId, expectedProvenancePath))
      .then((provenanceNode) => {
        if (!cancelled) {
          setProvenance({
            path: expectedProvenancePath,
            links: provenanceNode ? rawSourceLinksFor(expectedProvenancePath, provenanceNode.content) : []
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

  useEffect(() => {
    if (!expectedSkillManifestPath) {
      return;
    }
    let cancelled = false;
    import("@/lib/vfs-client")
      .then(({ readNode }) => readNode(canisterId, expectedSkillManifestPath))
      .then((manifestNode) => {
        if (!cancelled) {
          setSkillManifestState({
            path: expectedSkillManifestPath,
            manifest: manifestNode ? parseSkillManifest(manifestNode.content) : null
          });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSkillManifestState({ path: expectedSkillManifestPath, manifest: null });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId, expectedSkillManifestPath]);

  useEffect(() => {
    let cancelled = false;
    let cleanup = () => {};

    function refreshAccess(principal: string, authenticated: boolean) {
      import("@/lib/vfs-client")
        .then(({ myPathPolicyRoles, pathPolicy }) =>
          Promise.all([myPathPolicyRoles(canisterId, policyPath), pathPolicy(canisterId, policyPath)])
        )
        .then(([roles, policy]) => {
          if (!cancelled) {
            setSkillAccess({ principal, authenticated, roles, mode: policy.mode });
          }
        })
        .catch(() => {
          if (!cancelled) {
            setSkillAccess({ principal, authenticated, roles: [], mode: null });
          }
        });
    }

    import("@/lib/ii-auth")
      .then(({ subscribeAuth }) => {
        cleanup = subscribeAuth((state) => refreshAccess(state.principal, state.authenticated));
      })
      .catch(() => refreshAccess("2vxsx-fae", false));

    return () => {
      cancelled = true;
      cleanup();
    };
  }, [canisterId, policyPath]);

  useEffect(() => {
    if (!isSkillPath || !accessCapabilities.admin) {
      return;
    }
    let cancelled = false;
    import("@/lib/vfs-client")
      .then(({ pathPolicyEntries }) => pathPolicyEntries(canisterId, policyPath))
      .then((entries) => {
        if (!cancelled) setPolicyEntries(entries);
      })
      .catch((error: Error) => {
        if (!cancelled) setPolicyError(error.message);
      });
    return () => {
      cancelled = true;
    };
  }, [accessCapabilities.admin, canisterId, isSkillPath, policyPath]);

  async function updateAcl(action: "grant" | "revoke") {
    setPolicyError(null);
    try {
      Principal.fromText(policyPrincipal);
      const api = await import("@/lib/vfs-client");
      if (action === "grant") {
        await api.grantPathPolicyRole(canisterId, policyPath, policyPrincipal, policyRole);
      } else {
        await api.revokePathPolicyRole(canisterId, policyPath, policyPrincipal, policyRole);
      }
      setPolicyEntries(await api.pathPolicyEntries(canisterId, policyPath));
    } catch (error) {
      setPolicyError(error instanceof Error ? error.message : "Policy update failed");
    }
  }

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
      {isSkillPath ? (
        <InspectorCard title="Skill" icon={<PackageCheck size={15} />}>
          <Meta label="catalog" value={isPublicSkillPath ? "public" : "private"} />
          <Meta label="id" value={skillManifest?.id ?? null} />
          <Meta label="publisher" value={skillManifest?.publisher ?? null} />
          <Meta label="version" value={skillManifest?.version ?? null} />
          <Meta label="entry" value={skillManifest?.entry ?? null} />
          <Meta label="Access mode" value={skillAccess.mode} />
          <Meta label="Principal" value={skillAccess.authenticated ? skillAccess.principal : "anonymous (2vxsx-fae)"} />
          <button
            className="inline-flex w-fit items-center gap-1 rounded-md border border-line bg-white px-2 py-1 text-xs text-ink"
            type="button"
            onClick={() => navigator.clipboard.writeText(skillAccess.principal)}
          >
            <Copy size={13} />
            copy Principal
          </button>
          <Meta label="roles" value={skillAccess.roles.join(", ") || null} />
          <Meta label="Capabilities" value={formatSkillAccessCapabilities(accessCapabilities)} />
          <Meta label="Access hint" value={accessHint} />
          <Meta label="permissions" value={skillManifest ? formatRecord(skillManifest.permissions) : null} />
          <Meta label="knowledge" value={skillManifest?.knowledge.join(", ") || null} />
          <Meta label="provenance" value={skillManifest ? formatRecord(skillManifest.provenance) : null} />
          {skillManifest ? (
            <div className="flex flex-wrap gap-2">
              <button
                className="inline-flex w-fit items-center gap-1 rounded-md border border-line bg-white px-2 py-1 text-xs text-ink"
                type="button"
                onClick={() =>
                  navigator.clipboard.writeText(
                    isPublicSkillPath
                      ? `vfs-cli skill public install ${skillManifest.id} --output <dir> --lockfile --json`
                      : `vfs-cli skill install ${skillManifest.id} --output <dir> --lockfile --json`
                  )
                }
              >
                <Copy size={13} />
                copy install
              </button>
              {accessCapabilities.admin && !isPublicSkillPath ? (
                <button
                  className="inline-flex w-fit items-center gap-1 rounded-md border border-line bg-white px-2 py-1 text-xs text-ink"
                  type="button"
                  onClick={() => navigator.clipboard.writeText(`vfs-cli skill public promote ${skillManifest.id} --json`)}
                >
                  <Copy size={13} />
                  copy promote
                </button>
              ) : null}
              {accessCapabilities.admin && isPublicSkillPath ? (
                <button
                  className="inline-flex w-fit items-center gap-1 rounded-md border border-line bg-white px-2 py-1 text-xs text-ink"
                  type="button"
                  onClick={() => navigator.clipboard.writeText(`vfs-cli skill public revoke ${skillManifest.id} --json`)}
                >
                  <Copy size={13} />
                  copy revoke
                </button>
              ) : null}
            </div>
          ) : null}
        </InspectorCard>
      ) : null}
      {isSkillPath && accessCapabilities.admin ? (
        <InspectorCard title="Path Policy" icon={<Info size={15} />}>
          <Meta label="path" value={policyPath} />
          <Meta label="policy" value={skillAccess.mode} />
          <Meta label="roles" value="Admin, Writer, Reader" />
          <div className="space-y-2">
            {policyEntries.map((entry) => (
              <p key={entry.principal} className="truncate font-mono text-[11px] text-muted">
                {entry.principal} : {entry.roles.join(", ")}
              </p>
            ))}
          </div>
          <input
            className="w-full rounded-md border border-line px-2 py-1 font-mono text-xs"
            value={policyPrincipal}
            onChange={(event) => setPolicyPrincipal(event.target.value)}
            placeholder="principal"
          />
          <select className="w-full rounded-md border border-line px-2 py-1 text-xs" value={policyRole} onChange={(event) => setPolicyRole(event.target.value)}>
            <option>Reader</option>
            <option>Writer</option>
            <option>Admin</option>
          </select>
          <div className="flex gap-2">
            <button className="rounded-md border border-line bg-white px-2 py-1 text-xs" type="button" onClick={() => updateAcl("grant")}>
              Grant
            </button>
            <button className="rounded-md border border-line bg-white px-2 py-1 text-xs" type="button" onClick={() => updateAcl("revoke")}>
              Revoke
            </button>
          </div>
          {policyError ? <p className="text-xs text-red-700">{policyError}</p> : null}
        </InspectorCard>
      ) : null}
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
            {outgoingLinks.map((edge) => (
              <li key={`${edge.targetPath}-${edge.rawHref}`} className="truncate font-mono text-xs">
                <Link className="text-accent no-underline hover:underline" href={hrefForPath(canisterId, edge.targetPath)}>
                  {edge.targetPath}
                </Link>
                <p className="truncate text-[11px] text-muted">{edge.linkText || edge.rawHref}</p>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted">No outgoing links indexed.</p>
        )}
      </InspectorCard>
      <InspectorCard title="Incoming Links" icon={<GitBranch size={15} />}>
        {!node ? (
          <p className="text-xs text-muted">Select a file node to inspect backlinks.</p>
        ) : incomingLinks === null ? (
          <p className="text-xs text-muted">Loading backlinks...</p>
        ) : incomingError ? (
          <p className="text-xs text-red-700">{incomingError}</p>
        ) : incomingLinks.length > 0 ? (
          <ul className="space-y-1">
            {incomingLinks.map((edge) => (
              <li key={`${edge.sourcePath}-${edge.rawHref}`} className="truncate font-mono text-xs">
                <Link className="text-accent no-underline hover:underline" href={hrefForPath(canisterId, edge.sourcePath)}>
                  {edge.sourcePath}
                </Link>
                <p className="truncate text-[11px] text-muted">{edge.linkText || edge.rawHref}</p>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-muted">No backlinks indexed.</p>
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

function formatRecord(record: Record<string, string>): string | null {
  const entries = Object.entries(record);
  if (entries.length === 0) return null;
  return entries.map(([key, value]) => `${key}=${value}`).join(", ");
}
