"use client";

import Link from "next/link";
import dynamic from "next/dynamic";
import type { ReactNode } from "react";
import { useState } from "react";
import { FileText, Folder, Loader2 } from "lucide-react";
import { hrefForPath, hrefForSearch } from "@/lib/paths";
import { splitMarkdownPreviewSections } from "@/lib/markdown-sections";
import type { ChildNode, WikiNode } from "@/lib/types";
import type { LoadState, PathLoadState, ViewMode } from "@/lib/wiki-helpers";
import { ErrorBox } from "@/components/panel";

const LARGE_CONTENT_BYTES = 1024 * 1024;
const RAW_INITIAL_CHARS = 64 * 1024;
const RAW_LOAD_STEP_CHARS = 64 * 1024;
const MarkdownPreview = dynamic(() => import("@/components/markdown-preview").then((module) => module.MarkdownPreview), {
  ssr: false,
  loading: () => <p className="text-sm text-muted">Loading markdown preview...</p>
});

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
  if (node.data) return <PaneBody><NodeDocument node={node.data} view={view} canisterId={canisterId} /></PaneBody>;
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
          <Link className="rounded-lg border border-line bg-white px-3 py-2 no-underline" href={hrefForSearch(canisterId, path.split("/").filter(Boolean).at(-1) ?? path, "path")}>
            Search this path
          </Link>
        </div>
      </section>
    </div>
  );
}

function NodeDocument({ node, view, canisterId }: { node: WikiNode; view: ViewMode; canisterId: string }) {
  const contentBytes = new TextEncoder().encode(node.content).length;
  const isLargeContent = contentBytes > LARGE_CONTENT_BYTES;
  return (
    <article className="h-full overflow-auto px-6 py-6 md:px-10">
      {view === "raw" ? (
        <RawContent key={`${node.path}-${node.etag}`} content={node.content} isLargeContent={isLargeContent} contentBytes={contentBytes} />
      ) : isLargeContent ? (
        <LargeMarkdownPreview key={`${node.path}-${node.etag}`} content={node.content} contentBytes={contentBytes} canisterId={canisterId} nodePath={node.path} />
      ) : (
        <div className="markdown-body mx-auto max-w-3xl">
          <MarkdownPreview canisterId={canisterId} nodePath={node.path} content={node.content} />
        </div>
      )}
    </article>
  );
}

function LargeMarkdownPreview({
  content,
  contentBytes,
  canisterId,
  nodePath
}: {
  content: string;
  contentBytes: number;
  canisterId: string;
  nodePath: string;
}) {
  const sections = splitMarkdownPreviewSections(content);
  const [visibleSections, setVisibleSections] = useState(1);
  if (sections.length < 2) {
    return <LargeContentState contentBytes={contentBytes} canisterId={canisterId} nodePath={nodePath} reason="No section headings found." />;
  }
  const cappedVisibleSections = Math.min(visibleSections, sections.length);
  const showingFullPreview = cappedVisibleSections >= sections.length;
  const previewContent = sections.slice(0, cappedVisibleSections).join("\n");
  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-yellow-200 bg-yellow-50 p-3 text-sm text-yellow-900">
        <p>
          Large file: showing {cappedVisibleSections.toLocaleString()} of {sections.length.toLocaleString()} sections. Size: {contentBytes.toLocaleString()} bytes.
        </p>
        {showingFullPreview ? <p className="mt-2 font-medium">Showing full preview.</p> : null}
      </div>
      <div className="markdown-body mx-auto max-w-3xl">
        <MarkdownPreview canisterId={canisterId} nodePath={nodePath} content={previewContent} />
      </div>
      {!showingFullPreview ? (
        <button
          className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink hover:border-accent"
          type="button"
          onClick={() => setVisibleSections((current) => Math.min(current + 1, sections.length))}
        >
          Load next section
        </button>
      ) : null}
    </div>
  );
}

function RawContent({
  content,
  isLargeContent,
  contentBytes
}: {
  content: string;
  isLargeContent: boolean;
  contentBytes: number;
}) {
  const [visibleChars, setVisibleChars] = useState(isLargeContent ? RAW_INITIAL_CHARS : content.length);
  const cappedVisibleChars = Math.min(visibleChars, content.length);
  const visibleContent = isLargeContent ? content.slice(0, cappedVisibleChars) : content;
  const showingFullFile = cappedVisibleChars >= content.length;
  return (
    <div className="space-y-3">
      {isLargeContent ? (
        <div className="rounded-xl border border-yellow-200 bg-yellow-50 p-3 text-sm text-yellow-900">
          <p>
            Large file: showing {cappedVisibleChars.toLocaleString()} of {content.length.toLocaleString()} characters. Size: {contentBytes.toLocaleString()} bytes.
          </p>
          {showingFullFile ? <p className="mt-2 font-medium">Showing full file.</p> : null}
        </div>
      ) : null}
      <pre className="whitespace-pre-wrap rounded-xl border border-line bg-[#f7f3ea] p-5 font-mono text-sm leading-6">
        {visibleContent}
      </pre>
      {isLargeContent && !showingFullFile ? (
        <button
          className="rounded-lg border border-line bg-white px-3 py-2 text-sm text-ink hover:border-accent"
          type="button"
          onClick={() => setVisibleChars((current) => Math.min(current + RAW_LOAD_STEP_CHARS, content.length))}
        >
          Load more
        </button>
      ) : null}
    </div>
  );
}

function LargeContentState({
  contentBytes,
  canisterId,
  nodePath,
  reason
}: {
  contentBytes: number;
  canisterId: string;
  nodePath: string;
  reason?: string;
}) {
  return (
    <div className="mx-auto max-w-2xl rounded-2xl border border-line bg-paper p-6 text-sm">
      <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Large file</p>
      <h3 className="mt-3 text-2xl font-semibold tracking-[-0.04em]">Preview disabled</h3>
      <p className="mt-3 text-muted">
        This node is {contentBytes.toLocaleString()} bytes. Markdown preview is disabled to keep the browser responsive.
      </p>
      {reason ? <p className="mt-3 text-muted">{reason}</p> : null}
      <Link
        className="mt-5 inline-flex rounded-lg bg-accent px-3 py-2 text-white no-underline"
        href={hrefForPath(canisterId, nodePath, "raw")}
      >
        Open raw view
      </Link>
    </div>
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
