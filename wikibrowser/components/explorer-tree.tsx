"use client";

import type { Identity } from "@icp-sdk/core/agent";
import type { FormEvent } from "react";
import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { Check, ChevronDown, ChevronRight, FileText, Folder, FolderOpen, Plus, Trash2, X } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import { nodeRequestKey } from "@/lib/request-keys";
import type { ChildNode, DatabaseRole } from "@/lib/types";
import { canExpandChildNode, errorMessage, rootChild, type LoadState } from "@/lib/wiki-helpers";

type ExplorerMutationProps = {
  writeIdentity: Identity | null;
  currentDatabaseRole: DatabaseRole | null;
  databaseRoleError: string | null;
  onCreateMarkdownFile: (directoryPath: string, fileName: string) => Promise<boolean>;
  onDeleteMarkdownNode: (node: ChildNode) => Promise<boolean>;
};

export function ExplorerTree({
  canisterId,
  databaseId,
  selectedPath,
  autoExpandSelected = true,
  readIdentity,
  writeIdentity,
  currentDatabaseRole,
  databaseRoleError,
  readMode = null,
  childNodesCache,
  onCreateMarkdownFile,
  onDeleteMarkdownNode
}: {
  canisterId: string;
  databaseId: string;
  selectedPath: string;
  autoExpandSelected?: boolean;
  readIdentity: Identity | null;
  writeIdentity: Identity | null;
  currentDatabaseRole: DatabaseRole | null;
  databaseRoleError: string | null;
  readMode?: "anonymous" | null;
  childNodesCache: { current: Map<string, ChildNode[]> };
  onCreateMarkdownFile: (directoryPath: string, fileName: string) => Promise<boolean>;
  onDeleteMarkdownNode: (node: ChildNode) => Promise<boolean>;
}) {
  const readPrincipal = readIdentity?.getPrincipal().toText() ?? null;
  const mutationProps = { writeIdentity, currentDatabaseRole, databaseRoleError, onCreateMarkdownFile, onDeleteMarkdownNode };
  return (
    <div className="min-h-0 flex-1 space-y-1 overflow-auto p-2">
      <TreeNode key={`${canisterId}:${databaseId}:/Wiki:${readPrincipal ?? "anonymous"}`} canisterId={canisterId} databaseId={databaseId} node={rootChild("/Wiki")} selectedPath={selectedPath} depth={0} autoExpandSelected={autoExpandSelected} readIdentity={readIdentity} readMode={readMode} childNodesCache={childNodesCache} mutationProps={mutationProps} />
      <TreeNode key={`${canisterId}:${databaseId}:/Sources:${readPrincipal ?? "anonymous"}`} canisterId={canisterId} databaseId={databaseId} node={rootChild("/Sources")} selectedPath={selectedPath} depth={0} autoExpandSelected={autoExpandSelected} readIdentity={readIdentity} readMode={readMode} childNodesCache={childNodesCache} mutationProps={mutationProps} />
    </div>
  );
}

function TreeNode({
  canisterId,
  databaseId,
  node,
  selectedPath,
  depth,
  autoExpandSelected,
  readIdentity,
  readMode,
  childNodesCache,
  mutationProps
}: {
  canisterId: string;
  databaseId: string;
  node: ChildNode;
  selectedPath: string;
  depth: number;
  autoExpandSelected: boolean;
  readIdentity: Identity | null;
  readMode: "anonymous" | null;
  childNodesCache: { current: Map<string, ChildNode[]> };
  mutationProps: ExplorerMutationProps;
}) {
  const readPrincipal = readIdentity?.getPrincipal().toText() ?? null;
  const requestKey = nodeRequestKey(canisterId, databaseId, node.path, readPrincipal);
  const [expanded, setExpanded] = useState(autoExpandSelected && (node.path === selectedPath || selectedPath.startsWith(`${node.path}/`)));
  const [children, setChildren] = useState<LoadState<ChildNode[]>>(() => {
    const cached = childNodesCache.current.get(requestKey);
    return cached ? { data: cached, error: null, loading: false } : { data: null, error: null, loading: false };
  });
  const autoExpandedKey = useRef<string | null>(expanded ? selectedPath : null);
  const requestedKey = useRef<string | null>(null);
  const canExpand = canExpandChildNode(node);
  const selected = selectedPath === node.path;
  const selectedAncestor = node.path === selectedPath || selectedPath.startsWith(`${node.path}/`);
  const [createOpen, setCreateOpen] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<"create" | "delete" | null>(null);
  const disabledReason = writeDisabledReason(readMode, mutationProps);
  const canCreateMarkdown = isWikiDirectory(node);
  const canDeleteMarkdown = isWikiMarkdownFile(node) && Boolean(node.etag);

  useEffect(() => {
    if (!autoExpandSelected || !selectedAncestor || autoExpandedKey.current === selectedPath) return;
    const timeout = window.setTimeout(() => {
      autoExpandedKey.current = selectedPath;
      setExpanded(true);
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [autoExpandSelected, selectedAncestor, selectedPath]);

  useEffect(() => {
    if (!expanded || !canExpand || children.data || children.error || requestedKey.current === requestKey) return;
    const cached = childNodesCache.current.get(requestKey);
    if (cached) {
      let cancelled = false;
      Promise.resolve().then(() => {
        if (!cancelled) {
          setChildren({ data: cached, error: null, loading: false });
        }
      });
      return () => {
        cancelled = true;
      };
    }
    let cancelled = false;
    requestedKey.current = requestKey;
    Promise.resolve()
      .then(() => {
        if (cancelled) return null;
        setChildren({ data: null, error: null, loading: true });
        return import("@/lib/vfs-client");
      })
      .then((module) => {
        if (!module) return [];
        return module.listChildren(canisterId, databaseId, node.path, readIdentity ?? undefined);
      })
      .then((data) => {
        if (!cancelled) {
          childNodesCache.current.set(requestKey, data);
          setChildren({ data, error: null, loading: false });
        }
      })
      .catch((error: Error) => {
        if (!cancelled) {
          setChildren({ data: null, error: errorMessage(error), loading: false });
          requestedKey.current = null;
        }
      });
    return () => {
      cancelled = true;
      if (requestedKey.current === requestKey) requestedKey.current = null;
    };
  }, [canisterId, databaseId, canExpand, childNodesCache, children.data, children.error, expanded, node.path, readIdentity, requestKey]);

  async function submitCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setActionError(null);
    const normalizedName = normalizeMarkdownFileName(draftName);
    if (!normalizedName) {
      setActionError("Enter a Markdown file name, not a path.");
      return;
    }
    setBusyAction("create");
    try {
      const created = await mutationProps.onCreateMarkdownFile(node.path, normalizedName);
      if (created) {
        setCreateOpen(false);
        setDraftName("");
      }
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setBusyAction(null);
    }
  }

  async function deleteNode() {
    setActionError(null);
    setBusyAction("delete");
    try {
      await mutationProps.onDeleteMarkdownNode(node);
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <div>
      <div
        className={`group flex items-center gap-1 rounded-lg px-2 py-1.5 text-sm ${
          selected ? "bg-blue-50 text-accent" : "text-ink hover:bg-white"
        }`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
      >
        {canExpand ? <Toggle expanded={expanded} setExpanded={setExpanded} /> : <span className="w-[18px]" />}
        {directoryIcon(canExpand, expanded)}
        <Link
          className="min-w-0 flex-1 truncate no-underline"
          href={hrefForPath(canisterId, databaseId, node.path, undefined, undefined, undefined, undefined, readMode)}
        >
          {node.name}
        </Link>
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
          {canCreateMarkdown ? (
            <button
              aria-label={`New Markdown under ${node.path}`}
              className="rounded p-1 text-muted hover:bg-canvas disabled:cursor-not-allowed disabled:opacity-40"
              disabled={Boolean(disabledReason) || busyAction !== null}
              title={disabledReason ?? "New Markdown"}
              type="button"
              onClick={() => {
                setActionError(null);
                setCreateOpen(true);
                setExpanded(true);
              }}
            >
              <Plus size={13} />
            </button>
          ) : null}
          {canDeleteMarkdown ? (
            <button
              aria-label={`Delete ${node.path}`}
              className="rounded p-1 text-muted hover:bg-red-50 hover:text-red-700 disabled:cursor-not-allowed disabled:opacity-40"
              disabled={Boolean(disabledReason) || busyAction !== null}
              title={disabledReason ?? "Delete Markdown"}
              type="button"
              onClick={() => void deleteNode()}
            >
              <Trash2 size={13} />
            </button>
          ) : null}
        </div>
      </div>
      {createOpen ? (
        <form className="mt-1 flex items-center gap-1 px-2" style={{ paddingLeft: `${30 + depth * 16}px` }} onSubmit={submitCreate}>
          <input
            aria-label="Markdown file name"
            className="min-w-0 flex-1 rounded-md border border-line bg-white px-2 py-1 text-xs outline-none focus:border-accent"
            disabled={busyAction === "create"}
            placeholder="new-note.md"
            value={draftName}
            onChange={(event) => setDraftName(event.target.value)}
          />
          <button className="rounded p-1 text-accent hover:bg-blue-50 disabled:opacity-40" disabled={busyAction === "create"} title="Create" type="submit">
            <Check size={13} />
          </button>
          <button
            className="rounded p-1 text-muted hover:bg-canvas disabled:opacity-40"
            disabled={busyAction === "create"}
            title="Cancel"
            type="button"
            onClick={() => {
              setCreateOpen(false);
              setDraftName("");
              setActionError(null);
            }}
          >
            <X size={13} />
          </button>
        </form>
      ) : null}
      {actionError ? <TreeStatus depth={depth + 1} label={actionError} /> : null}
      {expanded && canExpand ? (
        <ChildrenList
          canisterId={canisterId}
          databaseId={databaseId}
          childrenState={children}
          depth={depth}
          selectedPath={selectedPath}
          autoExpandSelected={autoExpandSelected}
          readIdentity={readIdentity}
          readMode={readMode}
          childNodesCache={childNodesCache}
          mutationProps={mutationProps}
        />
      ) : null}
    </div>
  );
}

function Toggle({ expanded, setExpanded }: { expanded: boolean; setExpanded: (value: boolean) => void }) {
  return (
    <button
      className="rounded p-0.5 text-muted hover:bg-canvas"
      type="button"
      onClick={() => setExpanded(!expanded)}
      aria-label={expanded ? "Collapse directory" : "Expand directory"}
    >
      {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
    </button>
  );
}

function ChildrenList({
  canisterId,
  databaseId,
  childrenState,
  depth,
  selectedPath,
  autoExpandSelected,
  readIdentity,
  readMode,
  childNodesCache,
  mutationProps
}: {
  canisterId: string;
  databaseId: string;
  childrenState: LoadState<ChildNode[]>;
  depth: number;
  selectedPath: string;
  autoExpandSelected: boolean;
  readIdentity: Identity | null;
  readMode: "anonymous" | null;
  childNodesCache: { current: Map<string, ChildNode[]> };
  mutationProps: ExplorerMutationProps;
}) {
  return (
    <div>
      {!childrenState.data && !childrenState.error ? <TreeStatus depth={depth + 1} label="Loading" /> : null}
      {childrenState.error ? <TreeStatus depth={depth + 1} label={childrenState.error} /> : null}
      {childrenState.data?.map((child) => (
        <TreeNode
          key={child.path}
          canisterId={canisterId}
          databaseId={databaseId}
          node={child}
          selectedPath={selectedPath}
          depth={depth + 1}
          autoExpandSelected={autoExpandSelected}
          readIdentity={readIdentity}
          readMode={readMode}
          childNodesCache={childNodesCache}
          mutationProps={mutationProps}
        />
      ))}
    </div>
  );
}

function TreeStatus({ depth, label }: { depth: number; label: string }) {
  return (
    <div className="truncate px-2 py-1 font-mono text-[11px] text-muted" style={{ paddingLeft: `${26 + depth * 16}px` }}>
      {label}
    </div>
  );
}

function directoryIcon(isDirectory: boolean, expanded: boolean) {
  if (!isDirectory) return <FileText size={15} className="text-muted" />;
  return expanded ? <FolderOpen size={15} className="text-accent" /> : <Folder size={15} className="text-muted" />;
}

function writeDisabledReason(readMode: "anonymous" | null, props: ExplorerMutationProps): string | null {
  if (readMode === "anonymous") return "Switch to authenticated mode to write.";
  if (!props.writeIdentity) return "Login with Internet Identity to write.";
  if (props.databaseRoleError) return props.databaseRoleError;
  if (!props.currentDatabaseRole) return "Database role unavailable.";
  if (props.currentDatabaseRole !== "writer" && props.currentDatabaseRole !== "owner") return "Writer or owner access required.";
  return null;
}

function isWikiDirectory(node: ChildNode): boolean {
  return node.kind === "directory" && (node.path === "/Wiki" || node.path.startsWith("/Wiki/"));
}

function isWikiMarkdownFile(node: ChildNode): boolean {
  return node.kind === "file" && !node.isVirtual && node.path.startsWith("/Wiki/") && node.path.endsWith(".md");
}

function normalizeMarkdownFileName(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed || trimmed.includes("/") || trimmed === "." || trimmed === ".." || trimmed === ".md") return null;
  return trimmed.endsWith(".md") ? trimmed : `${trimmed}.md`;
}
