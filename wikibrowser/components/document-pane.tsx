"use client";

import Link from "next/link";
import type { ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { FileText, Folder, Loader2 } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode, WikiNode } from "@/lib/types";
import type { LoadState, PathLoadState, ViewMode } from "@/lib/wiki-helpers";
import { ErrorBox } from "@/components/panel";

export function DocumentHeader({
  path,
  view,
  onViewChange,
  isDirectory
}: {
  path: string;
  view: ViewMode;
  onViewChange: (view: ViewMode) => void;
  isDirectory: boolean;
}) {
  return (
    <div className="flex min-h-[60px] flex-col gap-2 border-b border-line bg-paper/80 px-5 py-3 md:flex-row md:items-center md:justify-between">
      <div className="min-w-0">
        <p className="font-mono text-xs text-muted">{isDirectory ? "directory" : "node"}</p>
        <h2 className="truncate text-lg font-semibold tracking-[-0.035em]">{path}</h2>
      </div>
      <div className="flex rounded-xl border border-line bg-white p-1 text-sm">
        <ViewButton active={view === "preview"} label="Preview" onClick={() => onViewChange("preview")} />
        <ViewButton active={view === "raw"} label="Raw" onClick={() => onViewChange("raw")} />
      </div>
    </div>
  );
}

export function DocumentPane({
  node,
  childrenState,
  view,
  canisterId
}: {
  node: PathLoadState<WikiNode>;
  childrenState: PathLoadState<ChildNode[]>;
  view: ViewMode;
  canisterId: string;
}) {
  if (node.loading && childrenState.loading) return <PaneBody><LoadingBlock /></PaneBody>;
  if (node.data) return <PaneBody><NodeDocument node={node.data} view={view} /></PaneBody>;
  if (childrenState.data) {
    return (
      <PaneBody>
        <DirectoryDocument childrenState={childrenState} canisterId={canisterId} />
      </PaneBody>
    );
  }
  if (isVfsNotFound(node.error, childrenState.error)) {
    return <PaneBody><NotFoundState path={node.path} canisterId={canisterId} /></PaneBody>;
  }
  return (
    <PaneBody className="p-6">
      <ErrorBox
        message={node.error ?? childrenState.error ?? "Unable to load node"}
        hint={node.hint ?? childrenState.hint}
      />
    </PaneBody>
  );
}

function PaneBody({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`min-h-0 flex-1 ${className}`}>{children}</div>;
}

function NotFoundState({
  path,
  canisterId
}: {
  path: string;
  canisterId: string;
}) {
  return (
    <div className="flex h-full items-center justify-center p-6">
      <section className="max-w-xl rounded-2xl border border-line bg-paper p-6 shadow-sm">
        <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Not found</p>
        <h3 className="mt-3 text-2xl font-semibold tracking-[-0.04em] text-ink">No wiki node at this path</h3>
        <p className="mt-3 break-all font-mono text-xs text-muted">{path}</p>
        <div className="mt-5 flex flex-wrap gap-2 text-sm">
          <Link
            className="rounded-lg bg-accent px-3 py-2 text-white no-underline"
            href={hrefForPath(canisterId, "/Wiki")}
          >
            Open /Wiki
          </Link>
          <Link
            className="rounded-lg border border-line bg-white px-3 py-2 no-underline"
            href={hrefForPath(canisterId, "/Sources")}
          >
            Open /Sources
          </Link>
          <Link className="rounded-lg border border-line bg-white px-3 py-2 no-underline" href={hrefForPath(canisterId, "/Wiki", undefined, "search", path.split("/").filter(Boolean).at(-1) ?? path, "path")}>
            Search this path
          </Link>
        </div>
      </section>
    </div>
  );
}

function NodeDocument({ node, view }: { node: WikiNode; view: ViewMode }) {
  return (
    <article className="h-full overflow-auto px-6 py-6 md:px-10">
      {view === "raw" ? (
        <pre className="whitespace-pre-wrap rounded-xl border border-line bg-[#f7f3ea] p-5 font-mono text-sm leading-6">
          {node.content}
        </pre>
      ) : (
        <div className="markdown-body mx-auto max-w-3xl">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{node.content}</ReactMarkdown>
        </div>
      )}
    </article>
  );
}

function DirectoryDocument({
  childrenState,
  canisterId
}: {
  childrenState: LoadState<ChildNode[]>;
  canisterId: string;
}) {
  return (
    <div className="h-full overflow-auto p-6">
      <div className="rounded-2xl border border-line bg-paper p-5">
        <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Directory</p>
        <h3 className="mt-2 text-2xl font-semibold tracking-[-0.04em]">Children</h3>
        <div className="mt-5 grid gap-2">
          {childrenState.data?.map((child) => (
            <Link
              key={child.path}
              href={hrefForPath(canisterId, child.path)}
              className="flex items-center justify-between rounded-xl border border-line bg-white px-4 py-3 text-sm no-underline hover:border-accent"
            >
              <span className="flex min-w-0 items-center gap-2">
                {child.kind === "directory" ? <Folder size={16} /> : <FileText size={16} />}
                <span className="truncate">{child.name}</span>
              </span>
              <span className="font-mono text-xs text-muted">{child.kind}</span>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}

function ViewButton({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      className={`rounded-lg px-3 py-1.5 ${active ? "bg-accent text-white" : "text-muted"}`}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function LoadingBlock() {
  return (
    <div className="flex h-full items-center justify-center text-muted">
      <Loader2 size={20} className="mr-2 animate-spin" />
      Loading wiki node
    </div>
  );
}

function isVfsNotFound(nodeError: string | null, childrenError: string | null): boolean {
  return Boolean(nodeError?.startsWith("node not found:") && childrenError?.startsWith("path not found:"));
}
