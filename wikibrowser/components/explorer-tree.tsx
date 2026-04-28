"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ChevronDown, ChevronRight, FileText, Folder, FolderOpen } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode } from "@/lib/types";
import { apiPath, errorMessage, fetchJson, rootChild, type LoadState } from "@/lib/wiki-helpers";

export function ExplorerTree({
  canisterId,
  selectedPath
}: {
  canisterId: string;
  selectedPath: string;
}) {
  return (
    <div className="min-h-0 flex-1 space-y-1 overflow-auto p-2">
      <TreeNode canisterId={canisterId} node={rootChild("/Wiki")} selectedPath={selectedPath} depth={0} />
      <TreeNode canisterId={canisterId} node={rootChild("/Sources")} selectedPath={selectedPath} depth={0} />
    </div>
  );
}

function TreeNode({
  canisterId,
  node,
  selectedPath,
  depth
}: {
  canisterId: string;
  node: ChildNode;
  selectedPath: string;
  depth: number;
}) {
  const [expanded, setExpanded] = useState(node.path === selectedPath || selectedPath.startsWith(`${node.path}/`));
  const [children, setChildren] = useState<LoadState<ChildNode[]>>({ data: null, error: null, loading: false });
  const requestedPath = useRef<string | null>(null);
  const isDirectory = node.kind === "directory";
  const selected = selectedPath === node.path;

  useEffect(() => {
    if (!expanded || !isDirectory || children.data || children.error || requestedPath.current === node.path) return;
    let cancelled = false;
    requestedPath.current = node.path;
    fetchJson<ChildNode[]>(apiPath(canisterId, "children", new URLSearchParams({ path: node.path })))
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
  }, [canisterId, children.data, children.error, expanded, isDirectory, node.path]);

  return (
    <div>
      <div
        className={`group flex items-center gap-1 rounded-lg px-2 py-1.5 text-sm ${
          selected ? "bg-blue-50 text-accent" : "text-ink hover:bg-white"
        }`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
      >
        {isDirectory ? <Toggle expanded={expanded} setExpanded={setExpanded} /> : <span className="w-[18px]" />}
        {directoryIcon(isDirectory, expanded)}
        <Link
          className="min-w-0 flex-1 truncate no-underline"
          href={hrefForPath(canisterId, node.path)}
        >
          {node.name}
        </Link>
      </div>
      {expanded && isDirectory ? (
        <ChildrenList
          canisterId={canisterId}
          childrenState={children}
          depth={depth}
          selectedPath={selectedPath}
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
  childrenState,
  depth,
  selectedPath
}: {
  canisterId: string;
  childrenState: LoadState<ChildNode[]>;
  depth: number;
  selectedPath: string;
}) {
  return (
    <div>
      {!childrenState.data && !childrenState.error ? <TreeStatus depth={depth + 1} label="Loading" /> : null}
      {childrenState.error ? <TreeStatus depth={depth + 1} label={childrenState.error} /> : null}
      {childrenState.data?.map((child) => (
        <TreeNode
          key={child.path}
          canisterId={canisterId}
          node={child}
          selectedPath={selectedPath}
          depth={depth + 1}
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
