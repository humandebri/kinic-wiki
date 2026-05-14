"use client";

import Link from "next/link";
import dynamic from "next/dynamic";
import type { ReactNode } from "react";
import { useState } from "react";
import type { Identity } from "@icp-sdk/core/agent";
import { FileText, Folder, Loader2 } from "lucide-react";
import { hrefForPath, hrefForSearch } from "@/lib/paths";
import { splitMarkdownPreviewSections } from "@/lib/markdown-sections";
import type { ChildNode, DatabaseRole, WikiNode } from "@/lib/types";
import type { LoadState, ModeTab, PathLoadState, ViewMode } from "@/lib/wiki-helpers";
import { ErrorBox } from "@/components/panel";
import type { EditorSaveState } from "@/components/markdown-editor";
import { MarkdownEditDocument } from "@/components/markdown-edit-document";

const LARGE_CONTENT_BYTES = 1024 * 1024;
const RAW_INITIAL_CHARS = 64 * 1024;
const RAW_LOAD_STEP_CHARS = 64 * 1024;
const MarkdownPreview = dynamic(() => import("@/components/markdown-preview").then((module) => module.MarkdownPreview), {
  ssr: false,
  loading: () => <p className="text-sm text-muted">Loading markdown preview...</p>
});

export type DocumentEditState = {
  dirty: boolean;
  saveState: EditorSaveState;
};

export function DocumentHeader({
  canisterId,
  databaseId,
  path,
  view,
  onViewChange,
  isDirectory,
  editState
}: {
  canisterId: string;
  databaseId: string;
  path: string;
  view: ViewMode;
  onViewChange: (view: ViewMode) => void;
  isDirectory: boolean;
  editState: DocumentEditState;
}) {
  return (
    <div className="flex min-h-[60px] flex-col gap-2 border-b border-line bg-paper/80 px-5 py-3 md:flex-row md:items-center md:justify-between">
      <div className="min-w-0">
        <p className="font-mono text-xs text-muted">{isDirectory ? "directory" : "node"}</p>
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <h2 className="truncate text-lg font-semibold tracking-[-0.035em]">{displayPath(path)}</h2>
          {view === "edit" ? <HeaderBadge label="Editing" tone="blue" /> : null}
          {view === "edit" && editState.dirty ? <HeaderBadge label="Unsaved" tone="yellow" /> : null}
          {view === "edit" && editState.saveState === "saving" ? <HeaderBadge label="Saving" tone="blue" /> : null}
          {view === "edit" && editState.saveState === "saved" ? <HeaderBadge label="Saved" tone="green" /> : null}
        </div>
      </div>
      <div className="flex rounded-xl border border-line bg-white p-1 text-sm">
        <ViewButton active={view === "preview"} label="Preview" onClick={() => onViewChange("preview")} />
        <ViewButton active={view === "raw"} label="Raw" onClick={() => onViewChange("raw")} />
        <ViewButton active={view === "edit"} label="Edit" onClick={() => onViewChange("edit")} />
      </div>
    </div>
  );
}

function displayPath(path: string): string {
  return path.startsWith("/Wiki/") ? path.slice("/Wiki/".length) : path;
}

export function DocumentPane({
  databaseId,
  node,
  childrenState,
  view,
  canisterId,
  authPrompt,
  authReady,
  onLogin,
  writeIdentity,
  currentDatabaseRole,
  databaseRoleError,
  onNodeSaved,
  onEditStateChange,
  tab,
  readMode = null
}: {
  node: PathLoadState<WikiNode>;
  childrenState: PathLoadState<ChildNode[]>;
  view: ViewMode;
  canisterId: string;
  databaseId: string;
  authPrompt?: "private" | "write" | null;
  authReady?: boolean;
  onLogin?: () => void;
  writeIdentity?: Identity | null;
  currentDatabaseRole?: DatabaseRole | null;
  databaseRoleError?: string | null;
  onNodeSaved?: () => Promise<WikiNode>;
  onEditStateChange?: (state: DocumentEditState) => void;
  tab?: ModeTab;
  readMode?: "anonymous" | null;
}) {
  if (node.loading && childrenState.loading) return <PaneBody><LoadingBlock /></PaneBody>;
  if (authPrompt && onLogin) {
    return <PaneBody className="p-6"><AuthRequiredState authReady={Boolean(authReady)} mode={authPrompt} onLogin={onLogin} /></PaneBody>;
  }
  if (node.data) {
    return (
      <PaneBody>
        <NodeDocument
          node={node.data}
          view={view}
          canisterId={canisterId}
          databaseId={databaseId}
          readMode={readMode}
          authReady={Boolean(authReady)}
          onLogin={onLogin}
          writeIdentity={writeIdentity ?? null}
          currentDatabaseRole={currentDatabaseRole ?? null}
          databaseRoleError={databaseRoleError ?? null}
          onNodeSaved={onNodeSaved}
          onEditStateChange={onEditStateChange}
          tab={tab}
        />
      </PaneBody>
    );
  }
  if (childrenState.data) {
    return (
      <PaneBody>
        <DirectoryDocument childrenState={childrenState} canisterId={canisterId} databaseId={databaseId} readMode={readMode} />
      </PaneBody>
    );
  }
  if (isVfsNotFound(node.error, childrenState.error)) {
    return <PaneBody><NotFoundState path={node.path} canisterId={canisterId} databaseId={databaseId} readMode={readMode} /></PaneBody>;
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

function AuthRequiredState({ authReady, mode, onLogin }: { authReady: boolean; mode: "private" | "write"; onLogin: () => void }) {
  const isWriteAction = mode === "write";
  return (
    <div className="flex h-full items-center justify-center">
      <section className="max-w-xl rounded-2xl border border-line bg-paper p-6 shadow-sm">
        <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">{isWriteAction ? "Write access" : "Private database"}</p>
        <h3 className="mt-3 text-2xl font-semibold tracking-[-0.04em] text-ink">Login required</h3>
        <p className="mt-3 text-sm leading-6 text-muted">
          {isWriteAction ? "Login with Internet Identity to queue URL ingest or save source clips for this database." : "This database is not public. Login with Internet Identity to read databases linked to your principal."}
        </p>
        <button
          className="mt-5 rounded-lg border border-accent bg-accent px-4 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
          disabled={!authReady}
          data-tid="login-button"
          type="button"
          onClick={onLogin}
        >
          Login with Internet Identity
        </button>
      </section>
    </div>
  );
}

function PaneBody({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`min-h-0 flex-1 ${className}`}>{children}</div>;
}

function NotFoundState({
  path,
  canisterId,
  databaseId,
  readMode
}: {
  path: string;
  canisterId: string;
  databaseId: string;
  readMode: "anonymous" | null;
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
            href={hrefForPath(canisterId, databaseId, "/Wiki", undefined, undefined, undefined, undefined, readMode)}
          >
            Open /Wiki
          </Link>
          <Link
            className="rounded-lg border border-line bg-white px-3 py-2 no-underline"
            href={hrefForPath(canisterId, databaseId, "/Sources", undefined, undefined, undefined, undefined, readMode)}
          >
            Open /Sources
          </Link>
          <Link className="rounded-lg border border-line bg-white px-3 py-2 no-underline" href={hrefForSearch(canisterId, databaseId, path.split("/").filter(Boolean).at(-1) ?? path, "path", readMode)}>
            Search this path
          </Link>
        </div>
      </section>
    </div>
  );
}

function NodeDocument({
  node,
  view,
  canisterId,
  databaseId,
  readMode,
  tab,
  authReady,
  onLogin,
  writeIdentity,
  currentDatabaseRole,
  databaseRoleError,
  onNodeSaved,
  onEditStateChange
}: {
  node: WikiNode;
  view: ViewMode;
  canisterId: string;
  databaseId: string;
  readMode: "anonymous" | null;
  tab?: ModeTab;
  authReady: boolean;
  onLogin?: () => void;
  writeIdentity: Identity | null;
  currentDatabaseRole: DatabaseRole | null;
  databaseRoleError: string | null;
  onNodeSaved?: () => Promise<WikiNode>;
  onEditStateChange?: (state: DocumentEditState) => void;
}) {
  const contentBytes = new TextEncoder().encode(node.content).length;
  const isLargeContent = contentBytes > LARGE_CONTENT_BYTES;
  if (view === "edit") {
    return (
      <EditDocument
        canisterId={canisterId}
        databaseId={databaseId}
        node={node}
        isLargeContent={isLargeContent}
        contentBytes={contentBytes}
        readMode={readMode}
        tab={tab}
        authReady={authReady}
        onLogin={onLogin}
        writeIdentity={writeIdentity}
        currentDatabaseRole={currentDatabaseRole}
        databaseRoleError={databaseRoleError}
        onNodeSaved={onNodeSaved}
        onEditStateChange={onEditStateChange}
      />
    );
  }
  return (
    <article className="h-full overflow-auto px-6 py-6 md:px-10">
      {view === "raw" ? (
        <RawContent key={`${node.path}-${node.etag}`} content={node.content} isLargeContent={isLargeContent} contentBytes={contentBytes} />
      ) : isLargeContent ? (
        <LargeMarkdownPreview key={`${node.path}:${node.etag}`} content={node.content} contentBytes={contentBytes} canisterId={canisterId} databaseId={databaseId} nodePath={node.path} readMode={readMode} />
      ) : (
        <div className="markdown-body mx-auto max-w-3xl">
          <MarkdownPreview canisterId={canisterId} databaseId={databaseId} nodePath={node.path} content={node.content} readMode={readMode} />
        </div>
      )}
    </article>
  );
}

function EditDocument({
  canisterId,
  databaseId,
  node,
  isLargeContent,
  contentBytes,
  readMode,
  tab,
  authReady,
  onLogin,
  writeIdentity,
  currentDatabaseRole,
  databaseRoleError,
  onNodeSaved,
  onEditStateChange
}: {
  canisterId: string;
  databaseId: string;
  node: WikiNode;
  isLargeContent: boolean;
  contentBytes: number;
  readMode: "anonymous" | null;
  tab?: ModeTab;
  authReady: boolean;
  onLogin?: () => void;
  writeIdentity: Identity | null;
  currentDatabaseRole: DatabaseRole | null;
  databaseRoleError: string | null;
  onNodeSaved?: () => Promise<WikiNode>;
  onEditStateChange?: (state: DocumentEditState) => void;
}) {
  const editable = node.kind === "file" && node.path.endsWith(".md") && !node.path.startsWith("/Sources/raw/");
  if (!editable) {
    return <EditorUnavailable title="Read-only node" message="Only existing Markdown file nodes outside /Sources/raw can be edited in the browser." />;
  }
  if (readMode === "anonymous") {
    return (
      <EditorUnavailable
        title="Authenticated mode required"
        message="This page is using anonymous read mode. Switch to authenticated mode before editing."
        actionHref={hrefForPath(canisterId, databaseId, node.path, "edit", tab)}
        actionLabel="Use authenticated mode"
      />
    );
  }
  if (!writeIdentity) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <section className="max-w-xl rounded-2xl border border-line bg-paper p-6 shadow-sm">
          <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Edit access</p>
          <h3 className="mt-3 text-2xl font-semibold tracking-[-0.04em] text-ink">Login required</h3>
          <p className="mt-3 text-sm leading-6 text-muted">Login with Internet Identity to save Markdown changes.</p>
          {onLogin ? (
            <button
              className="mt-5 rounded-lg border border-accent bg-accent px-4 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
              disabled={!authReady}
              type="button"
              onClick={onLogin}
            >
              Login with Internet Identity
            </button>
          ) : null}
        </section>
      </div>
    );
  }
  if (databaseRoleError) {
    return <EditorUnavailable title="Database role unavailable" message={databaseRoleError} />;
  }
  if (!currentDatabaseRole) {
    return <EditorUnavailable title="Database role unavailable" message="Reload database membership before editing this Markdown node." />;
  }
  if (currentDatabaseRole !== "writer" && currentDatabaseRole !== "owner") {
    return <EditorUnavailable title="Writer or owner access required" message="This principal can read the database but cannot save Markdown changes." />;
  }
  if (!onNodeSaved) {
    return <EditorUnavailable title="Save unavailable" message="The browser cannot refresh this node after saving." />;
  }
  return (
    <MarkdownEditDocument
      canisterId={canisterId}
      databaseId={databaseId}
      node={node}
      isLargeContent={isLargeContent}
      contentBytes={contentBytes}
      writeIdentity={writeIdentity}
      onNodeSaved={onNodeSaved}
      onEditStateChange={onEditStateChange}
    />
  );
}

function LargeMarkdownPreview({
  content,
  contentBytes,
  canisterId,
  databaseId,
  nodePath,
  readMode
}: {
  content: string;
  contentBytes: number;
  canisterId: string;
  databaseId: string;
  nodePath: string;
  readMode: "anonymous" | null;
}) {
  const sections = splitMarkdownPreviewSections(content);
  const [visibleSections, setVisibleSections] = useState(1);
  if (sections.length < 2) {
    return <LargeContentState contentBytes={contentBytes} canisterId={canisterId} databaseId={databaseId} nodePath={nodePath} readMode={readMode} reason="No section headings found." />;
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
        <MarkdownPreview canisterId={canisterId} databaseId={databaseId} nodePath={nodePath} content={previewContent} readMode={readMode} />
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
  databaseId,
  nodePath,
  readMode,
  reason
}: {
  contentBytes: number;
  canisterId: string;
  databaseId: string;
  nodePath: string;
  readMode: "anonymous" | null;
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
        href={hrefForPath(canisterId, databaseId, nodePath, "raw", undefined, undefined, undefined, readMode)}
      >
        Open raw view
      </Link>
    </div>
  );
}

function DirectoryDocument({
  childrenState,
  canisterId,
  databaseId,
  readMode
}: {
  childrenState: LoadState<ChildNode[]>;
  canisterId: string;
  databaseId: string;
  readMode: "anonymous" | null;
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
              href={hrefForPath(canisterId, databaseId, child.path, undefined, undefined, undefined, undefined, readMode)}
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

function HeaderBadge({ label, tone }: { label: string; tone: "blue" | "green" | "yellow" }) {
  const className =
    tone === "green"
      ? "bg-emerald-100 text-emerald-900"
      : tone === "yellow"
        ? "bg-yellow-100 text-yellow-900"
        : "bg-blue-100 text-blue-900";
  return <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${className}`}>{label}</span>;
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

function EditorUnavailable({ title, message, actionHref, actionLabel }: { title: string; message: string; actionHref?: string; actionLabel?: string }) {
  return (
    <div className="flex h-full items-center justify-center p-6">
      <section className="max-w-xl rounded-2xl border border-line bg-paper p-6 shadow-sm">
        <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Edit unavailable</p>
        <h3 className="mt-3 text-2xl font-semibold tracking-[-0.04em] text-ink">{title}</h3>
        <p className="mt-3 text-sm leading-6 text-muted">{message}</p>
        {actionHref && actionLabel ? (
          <Link className="mt-5 inline-flex rounded-lg border border-accent bg-accent px-4 py-2 text-sm font-medium text-white no-underline" href={actionHref}>
            {actionLabel}
          </Link>
        ) : null}
      </section>
    </div>
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
