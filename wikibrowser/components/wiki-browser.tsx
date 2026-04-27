"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  GitBranch,
  Info,
  Loader2,
  PanelRight,
  Search,
  Sparkles
} from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode, WikiNode } from "@/lib/types";

type ViewMode = "preview" | "raw";
type LoadState<T> = { data: T | null; error: string | null; loading: boolean };
type PathLoadState<T> = LoadState<T> & { path: string };

type WikiBrowserProps = {
  canisterId: string;
  selectedPath: string;
  initialView: ViewMode;
};

export function WikiBrowser({ canisterId, selectedPath, initialView }: WikiBrowserProps) {
  const [view, setView] = useState<ViewMode>(initialView);
  const [node, setNode] = useState<PathLoadState<WikiNode>>({
    path: selectedPath,
    data: null,
    error: null,
    loading: true
  });
  const [childNodes, setChildNodes] = useState<PathLoadState<ChildNode[]>>({
    path: selectedPath,
    data: null,
    error: null,
    loading: true
  });

  useEffect(() => {
    let cancelled = false;

    fetchJson<WikiNode>(apiPath(canisterId, "node", selectedPath))
      .then((data) => {
        if (!cancelled) {
          setNode({ path: selectedPath, data, error: null, loading: false });
          setChildNodes({ path: selectedPath, data: [], error: null, loading: false });
        }
      })
      .catch((nodeError: Error) => {
        fetchJson<ChildNode[]>(apiPath(canisterId, "children", selectedPath))
          .then((data) => {
            if (!cancelled) {
              setNode({ path: selectedPath, data: null, error: null, loading: false });
              setChildNodes({ path: selectedPath, data, error: null, loading: false });
            }
          })
          .catch((childrenError: Error) => {
            if (!cancelled) {
              setNode({ path: selectedPath, data: null, error: nodeError.message, loading: false });
              setChildNodes({ path: selectedPath, data: null, error: childrenError.message, loading: false });
            }
          });
      });

    return () => {
      cancelled = true;
    };
  }, [canisterId, selectedPath]);

  const currentNode = node.path === selectedPath ? node : loadingState<WikiNode>(selectedPath);
  const currentChildren =
    childNodes.path === selectedPath ? childNodes : loadingState<ChildNode[]>(selectedPath);
  const outgoingLinks = useMemo(() => extractMarkdownLinks(currentNode.data?.content ?? ""), [currentNode.data]);
  const noteRole = inferNoteRole(selectedPath);

  return (
    <main className="flex min-h-screen flex-col bg-canvas text-ink">
      <TopBar canisterId={canisterId} selectedPath={selectedPath} />
      <section className="grid min-h-0 flex-1 grid-cols-1 gap-3 p-3 lg:grid-cols-[320px_minmax(0,1fr)_320px]">
        <aside className="min-h-[280px] overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<GitBranch size={15} />} title="Explorer" subtitle="lazy VFS tree" />
          <Explorer canisterId={canisterId} selectedPath={selectedPath} />
        </aside>

        <section className="min-h-[60vh] overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
          <DocumentHeader
            path={selectedPath}
            view={view}
            onViewChange={setView}
            isDirectory={!currentNode.data && Boolean(currentChildren.data)}
          />
          <DocumentPane node={currentNode} childrenState={currentChildren} view={view} canisterId={canisterId} />
        </section>

        <aside className="min-h-[280px] overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<PanelRight size={15} />} title="Inspector" subtitle="read-only metadata" />
          <Inspector
            path={selectedPath}
            node={currentNode.data}
            childNodes={currentChildren.data ?? []}
            noteRole={noteRole}
            outgoingLinks={outgoingLinks}
          />
        </aside>
      </section>
    </main>
  );
}

function TopBar({ canisterId, selectedPath }: { canisterId: string; selectedPath: string }) {
  return (
    <header className="flex flex-col gap-3 border-b border-line bg-paper/80 px-4 py-3 backdrop-blur md:flex-row md:items-center md:justify-between">
      <div>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted">Wiki Canister Browser</p>
        <h1 className="mt-1 text-lg font-semibold tracking-[-0.03em]">Knowledge IDE</h1>
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-2 md:max-w-2xl md:flex-row">
        <div className="flex min-w-0 flex-1 items-center gap-2 rounded-xl border border-line bg-white px-3 py-2 text-sm text-muted">
          <Search size={15} />
          <span className="truncate">Search arrives in Phase 4</span>
        </div>
        <div className="rounded-xl border border-line bg-white px-3 py-2 font-mono text-xs text-muted">
          <span className="text-ink">{canisterId}</span>
          <span className="mx-2 text-line">/</span>
          <span>{selectedPath}</span>
        </div>
      </div>
    </header>
  );
}

function PanelHeader({
  icon,
  title,
  subtitle
}: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
}) {
  return (
    <div className="flex items-center gap-2 border-b border-line px-4 py-3">
      <span className="text-accent">{icon}</span>
      <div>
        <h2 className="text-sm font-semibold">{title}</h2>
        <p className="text-xs text-muted">{subtitle}</p>
      </div>
    </div>
  );
}

function Explorer({ canisterId, selectedPath }: { canisterId: string; selectedPath: string }) {
  return (
    <div className="space-y-1 overflow-auto p-2">
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
  const [expanded, setExpanded] = useState(
    node.path === selectedPath || selectedPath.startsWith(`${node.path}/`)
  );
  const [children, setChildren] = useState<LoadState<ChildNode[]>>({
    data: null,
    error: null,
    loading: false
  });
  const requestedPath = useRef<string | null>(null);
  const isDirectory = node.kind === "directory";
  const selected = selectedPath === node.path;

  useEffect(() => {
    if (!expanded || !isDirectory || children.data || children.error || requestedPath.current === node.path) {
      return;
    }
    let cancelled = false;
    requestedPath.current = node.path;
    fetchJson<ChildNode[]>(apiPath(canisterId, "children", node.path))
      .then((data) => {
        if (!cancelled) {
          setChildren({ data, error: null, loading: false });
        }
      })
      .catch((error: Error) => {
        if (!cancelled) {
          setChildren({ data: null, error: error.message, loading: false });
          requestedPath.current = null;
        }
      });
    return () => {
      cancelled = true;
      if (requestedPath.current === node.path) {
        requestedPath.current = null;
      }
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
        {isDirectory ? (
          <button
            className="rounded p-0.5 text-muted hover:bg-canvas"
            type="button"
            onClick={() => setExpanded((value) => !value)}
            aria-label={expanded ? "Collapse directory" : "Expand directory"}
          >
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
        ) : (
          <span className="w-[18px]" />
        )}
        {isDirectory ? (
          expanded ? (
            <FolderOpen size={15} className="text-accent" />
          ) : (
            <Folder size={15} className="text-muted" />
          )
        ) : (
          <FileText size={15} className="text-muted" />
        )}
        <Link className="min-w-0 flex-1 truncate no-underline" href={hrefForPath(canisterId, node.path)}>
          {node.name}
        </Link>
      </div>
      {expanded && isDirectory ? (
        <div>
          {expanded && isDirectory && !children.data && !children.error ? (
            <TreeStatus depth={depth + 1} label="Loading" />
          ) : null}
          {children.error ? <TreeStatus depth={depth + 1} label={children.error} /> : null}
          {children.data?.map((child) => (
            <TreeNode
              key={child.path}
              canisterId={canisterId}
              node={child}
              selectedPath={selectedPath}
              depth={depth + 1}
            />
          ))}
        </div>
      ) : null}
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

function DocumentHeader({
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
    <div className="flex flex-col gap-3 border-b border-line bg-paper/80 px-5 py-4 md:flex-row md:items-center md:justify-between">
      <div className="min-w-0">
        <p className="font-mono text-xs text-muted">{isDirectory ? "directory" : "node"}</p>
        <h2 className="truncate text-xl font-semibold tracking-[-0.035em]">{path}</h2>
      </div>
      <div className="flex rounded-xl border border-line bg-white p-1 text-sm">
        <button
          type="button"
          className={`rounded-lg px-3 py-1.5 ${view === "preview" ? "bg-accent text-white" : "text-muted"}`}
          onClick={() => onViewChange("preview")}
        >
          Preview
        </button>
        <button
          type="button"
          className={`rounded-lg px-3 py-1.5 ${view === "raw" ? "bg-accent text-white" : "text-muted"}`}
          onClick={() => onViewChange("raw")}
        >
          Raw
        </button>
      </div>
    </div>
  );
}

function DocumentPane({
  node,
  childrenState,
  view,
  canisterId
}: {
  node: LoadState<WikiNode>;
  childrenState: LoadState<ChildNode[]>;
  view: ViewMode;
  canisterId: string;
}) {
  if (node.loading && childrenState.loading) {
    return <LoadingBlock />;
  }
  if (node.data) {
    return (
      <article className="h-[calc(100vh-152px)] overflow-auto px-6 py-6 md:px-10">
        {view === "raw" ? (
          <pre className="whitespace-pre-wrap rounded-xl border border-line bg-[#f7f3ea] p-5 font-mono text-sm leading-6">
            {node.data.content}
          </pre>
        ) : (
          <div className="markdown-body mx-auto max-w-3xl">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{node.data.content}</ReactMarkdown>
          </div>
        )}
      </article>
    );
  }
  if (childrenState.data) {
    return (
      <div className="h-[calc(100vh-152px)] overflow-auto p-6">
        <div className="rounded-2xl border border-line bg-paper p-5">
          <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Directory</p>
          <h3 className="mt-2 text-2xl font-semibold tracking-[-0.04em]">Children</h3>
          <div className="mt-5 grid gap-2">
            {childrenState.data.map((child) => (
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
  return (
    <div className="p-6">
      <div className="rounded-2xl border border-red-200 bg-red-50 p-5 text-sm text-red-700">
        {node.error ?? childrenState.error ?? "Unable to load node"}
      </div>
    </div>
  );
}

function LoadingBlock() {
  return (
    <div className="flex h-[calc(100vh-152px)] items-center justify-center text-muted">
      <Loader2 size={20} className="mr-2 animate-spin" />
      Loading wiki node
    </div>
  );
}

function Inspector({
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

function InspectorCard({
  title,
  icon,
  children
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-line bg-white p-4">
      <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold">
        <span className="text-accent">{icon}</span>
        {title}
      </h3>
      <div className="space-y-2">{children}</div>
    </section>
  );
}

function Meta({ label, value }: { label: string; value: string | null }) {
  return (
    <div>
      <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted">{label}</div>
      <div className="mt-1 break-all font-mono text-xs text-ink">{value ?? "-"}</div>
    </div>
  );
}

function rootChild(path: "/Wiki" | "/Sources"): ChildNode {
  return {
    path,
    name: path.slice(1),
    kind: "directory",
    updatedAt: null,
    etag: null,
    sizeBytes: null,
    isVirtual: true
  };
}

function apiPath(canisterId: string, endpoint: "node" | "children", path: string): string {
  return `/api/site/${encodeURIComponent(canisterId)}/${endpoint}?path=${encodeURIComponent(path)}`;
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  const body = (await response.json()) as unknown;
  if (!response.ok) {
    throw new Error(isErrorBody(body) ? body.error : `request failed: ${response.status}`);
  }
  return body as T;
}

function isErrorBody(value: unknown): value is { error: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "error" in value &&
    typeof value.error === "string"
  );
}

function loadingState<T>(path: string): PathLoadState<T> {
  return { path, data: null, error: null, loading: true };
}

function inferNoteRole(path: string): string {
  const name = path.split("/").at(-1) ?? "";
  if (name === "facts.md") return "facts";
  if (name === "events.md") return "events";
  if (name === "plans.md") return "plans";
  if (name === "summary.md") return "summary";
  if (name === "open_questions.md") return "open_questions";
  if (path.startsWith("/Sources/raw")) return "raw_source";
  if (path.endsWith(".md")) return "markdown_note";
  return "directory";
}

function extractMarkdownLinks(content: string): string[] {
  const links = new Set<string>();
  const inlinePattern = /\[[^\]]+\]\(([^)]+)\)/g;
  const wikiPattern = /\[\[([^\]]+)\]\]/g;
  for (const match of content.matchAll(inlinePattern)) {
    links.add(match[1] ?? "");
  }
  for (const match of content.matchAll(wikiPattern)) {
    links.add(match[1] ?? "");
  }
  return [...links].filter(Boolean).slice(0, 20);
}
