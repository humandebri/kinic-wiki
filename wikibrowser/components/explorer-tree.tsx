"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ChevronDown, ChevronRight, FileText, Folder, FolderOpen } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode } from "@/lib/types";
import { canExpandChildNode, errorMessage, rootChild, type LoadState } from "@/lib/wiki-helpers";

export function ExplorerTree({
  canisterId,
  databaseId,
  selectedPath,
  autoExpandSelected = true
}: {
  canisterId: string;
  databaseId: string;
  selectedPath: string;
  autoExpandSelected?: boolean;
}) {
  return (
    <div className="min-h-0 flex-1 space-y-1 overflow-auto p-2">
      <TreeNode canisterId={canisterId} databaseId={databaseId} node={rootChild("/Wiki")} selectedPath={selectedPath} depth={0} autoExpandSelected={autoExpandSelected} />
      <TreeNode canisterId={canisterId} databaseId={databaseId} node={rootChild("/Sources")} selectedPath={selectedPath} depth={0} autoExpandSelected={autoExpandSelected} />
    </div>
  );
}

function TreeNode({
  canisterId,
  databaseId,
  node,
  selectedPath,
  depth,
  autoExpandSelected
}: {
  canisterId: string;
  databaseId: string;
  node: ChildNode;
  selectedPath: string;
  depth: number;
  autoExpandSelected: boolean;
}) {
  const [expanded, setExpanded] = useState(autoExpandSelected && (node.path === selectedPath || selectedPath.startsWith(`${node.path}/`)));
  const [children, setChildren] = useState<LoadState<ChildNode[]>>({ data: null, error: null, loading: false });
  const requestedPath = useRef<string | null>(null);
  const canExpand = canExpandChildNode(node);
  const selected = selectedPath === node.path;

  useEffect(() => {
    if (!expanded || !canExpand || children.data || children.error || requestedPath.current === node.path) return;
    let cancelled = false;
    requestedPath.current = node.path;
    import("@/lib/vfs-client")
      .then(({ listChildren }) => listChildren(canisterId, databaseId, node.path))
      .then((data) => {
        if (!cancelled) setChildren({ data, error: null, loading: false });
      })
      .catch((error: Error) => {
        if (!cancelled) {
          setChildren({ data: null, error: errorMessage(error), loading: false });
          requestedPath.current = null;
        }
      });
    return () => {
      cancelled = true;
      if (requestedPath.current === node.path) requestedPath.current = null;
    };
  }, [canisterId, databaseId, canExpand, children.data, children.error, expanded, node.path]);

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
          href={hrefForPath(canisterId, databaseId, node.path)}
        >
          {node.name}
        </Link>
      </div>
      {expanded && canExpand ? (
        <ChildrenList
          canisterId={canisterId}
          databaseId={databaseId}
          childrenState={children}
          depth={depth}
          selectedPath={selectedPath}
          autoExpandSelected={autoExpandSelected}
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
  autoExpandSelected
}: {
  canisterId: string;
  databaseId: string;
  childrenState: LoadState<ChildNode[]>;
  depth: number;
  selectedPath: string;
  autoExpandSelected: boolean;
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
