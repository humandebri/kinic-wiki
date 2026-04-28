"use client";

import type { FormEvent } from "react";
import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { GitBranch, PanelRight, Search } from "lucide-react";
import { DocumentHeader, DocumentPane } from "@/components/document-pane";
import { ExplorerTree } from "@/components/explorer-tree";
import { Inspector } from "@/components/inspector";
import { LintPanel } from "@/components/lint-panel";
import { PanelHeader } from "@/components/panel";
import { RecentPanel } from "@/components/recent-panel";
import { SearchPanel } from "@/components/search-panel";
import { hrefForPath } from "@/lib/paths";
import type { ChildNode, WikiNode } from "@/lib/types";
import {
  apiPath,
  errorHint,
  errorMessage,
  extractMarkdownLinks,
  fetchJson,
  inferNoteRole,
  isNotFoundError,
  loadingState,
  type ModeTab,
  type PathLoadState,
  type ViewMode
} from "@/lib/wiki-helpers";

const SIDEBAR_TABS: ModeTab[] = ["explorer", "recent", "lint"];

type WikiBrowserProps = {
  canisterId: string;
};

export function WikiBrowser({ canisterId }: WikiBrowserProps) {
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const selectedPath = useMemo(() => pathFromSitePathname(canisterId, pathname), [canisterId, pathname]);
  const view = parseView(searchParams.get("view"));
  const tab = parseTab(searchParams.get("tab"));
  const query = searchParams.get("q") ?? "";
  const searchKind = parseSearchKind(searchParams.get("kind"));
  const [node, setNode] = useState<PathLoadState<WikiNode>>(loadingState(selectedPath));
  const [childNodes, setChildNodes] = useState<PathLoadState<ChildNode[]>>(loadingState(selectedPath));

  useEffect(() => {
    let cancelled = false;
    const nodeUrl = apiPath(canisterId, "node", new URLSearchParams({ path: selectedPath }));
    const childrenUrl = apiPath(canisterId, "children", new URLSearchParams({ path: selectedPath }));
    fetchJson<WikiNode>(nodeUrl)
      .then((data) => {
        if (!cancelled) {
          setNode({ path: selectedPath, data, error: null, loading: false });
          setChildNodes({ path: selectedPath, data: [], error: null, loading: false });
        }
      })
      .catch((nodeError: Error) => {
        if (!isNotFoundError(nodeError)) {
          if (!cancelled) {
            setNode({ path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
            setChildNodes({ path: selectedPath, data: null, error: null, loading: false });
          }
          return;
        }
        fetchJson<ChildNode[]>(childrenUrl)
          .then((data) => {
            if (!cancelled) {
              if (data.length === 0 && looksLikeFilePath(selectedPath)) {
                setNode({ path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
                setChildNodes({ path: selectedPath, data: null, error: `path not found: ${selectedPath}`, loading: false });
              } else {
                setNode({ path: selectedPath, data: null, error: null, loading: false });
                setChildNodes({ path: selectedPath, data, error: null, loading: false });
              }
            }
          })
          .catch((childrenError: Error) => {
            if (!cancelled) {
              setNode({ path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
              setChildNodes({ path: selectedPath, data: null, error: errorMessage(childrenError), hint: errorHint(childrenError), loading: false });
            }
          });
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId, selectedPath]);

  const currentNode = node.path === selectedPath ? node : loadingState<WikiNode>(selectedPath);
  const currentChildren = childNodes.path === selectedPath ? childNodes : loadingState<ChildNode[]>(selectedPath);
  const outgoingLinks = useMemo(() => extractMarkdownLinks(currentNode.data?.content ?? ""), [currentNode.data]);
  const noteRole = inferNoteRole(selectedPath);

  return (
    <main className="flex h-screen flex-col overflow-hidden bg-canvas text-ink">
      <TopBar
        canisterId={canisterId}
        selectedPath={selectedPath}
        query={query}
        searchKind={searchKind}
      />
      <section className="grid min-h-0 flex-1 grid-cols-1 gap-3 p-3 lg:grid-cols-[320px_minmax(0,1fr)_320px]">
        <aside className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<GitBranch size={15} />} title={tabTitle(tab)} />
          <ModeTabs canisterId={canisterId} selectedPath={selectedPath} tab={tab} />
          <LeftPane
            tab={tab}
            canisterId={canisterId}
            selectedPath={selectedPath}
            query={query}
            searchKind={searchKind}
            node={currentNode.data}
          />
        </aside>
        <section className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
          <DocumentHeader
            path={selectedPath}
            view={view}
            onViewChange={(nextView) => {
              router.replace(hrefForPath(canisterId, selectedPath, nextView, tab, query, searchKind));
            }}
            isDirectory={!currentNode.data && Boolean(currentChildren.data)}
          />
          <DocumentPane
            node={currentNode}
            childrenState={currentChildren}
            view={view}
            canisterId={canisterId}
          />
        </section>
        <aside className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<PanelRight size={15} />} title="Inspector" subtitle="metadata and hints" />
          <Inspector
            canisterId={canisterId}
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

function LeftPane({
  tab,
  canisterId,
  selectedPath,
  query,
  searchKind,
  node
}: {
  tab: ModeTab;
  canisterId: string;
  selectedPath: string;
  query: string;
  searchKind: "path" | "full";
  node: WikiNode | null;
}) {
  if (tab === "search") {
    return (
      <SearchPanel
        canisterId={canisterId}
        selectedPath={selectedPath}
        query={query}
        initialKind={searchKind}
      />
    );
  }
  if (tab === "recent") return <RecentPanel canisterId={canisterId} />;
  if (tab === "lint") return <LintPanel path={selectedPath} node={node} canisterId={canisterId} />;
  return <ExplorerTree canisterId={canisterId} selectedPath={selectedPath} />;
}

function TopBar({
  canisterId,
  selectedPath,
  query,
  searchKind
}: {
  canisterId: string;
  selectedPath: string;
  query: string;
  searchKind: "path" | "full";
}) {
  return (
    <header className="flex min-h-[52px] items-center gap-4 border-b border-line bg-paper/80 px-3 py-2 backdrop-blur">
      <div className="w-[168px] shrink-0">
        <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">Wiki Canister Browser</p>
        <h1 className="text-base font-semibold leading-tight tracking-[-0.03em]">Knowledge IDE</h1>
      </div>
      <div className="flex min-w-0 flex-1 items-center justify-end">
        <HeaderSearch canisterId={canisterId} selectedPath={selectedPath} query={query} searchKind={searchKind} />
      </div>
    </header>
  );
}

function HeaderSearch({
  canisterId,
  selectedPath,
  query,
  searchKind
}: {
  canisterId: string;
  selectedPath: string;
  query: string;
  searchKind: "path" | "full";
}) {
  const router = useRouter();
  const draftKey = `${query}\n${searchKind}`;
  const [draft, setDraft] = useState({ key: draftKey, text: query, kind: searchKind });
  const text = draft.key === draftKey ? draft.text : query;
  const kind = draft.key === draftKey ? draft.kind : searchKind;

  function submitSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    router.replace(hrefForPath(canisterId, selectedPath, undefined, "search", text.trim(), kind));
  }

  return (
    <form className="ml-auto flex w-full max-w-[720px] items-center gap-2 rounded-xl border border-line bg-white px-2 py-1.5 text-sm" onSubmit={submitSearch}>
      <div className="flex shrink-0 rounded-lg border border-line bg-paper p-1 text-xs">
        <SearchKindButton active={kind === "path"} label="Path" onClick={() => setDraft({ key: draftKey, text, kind: "path" })} />
        <SearchKindButton active={kind === "full"} label="Full text" onClick={() => setDraft({ key: draftKey, text, kind: "full" })} />
      </div>
      <Search size={15} className="shrink-0 text-muted" />
      <input
        className="min-w-0 flex-1 bg-transparent py-1 outline-none placeholder:text-muted"
        value={text}
        onChange={(event) => setDraft({ key: draftKey, text: event.target.value, kind })}
        placeholder="Search wiki"
        aria-label="Search wiki"
      />
      <button className="rounded-lg bg-accent px-3 py-1.5 text-white" type="submit">
        Search
      </button>
    </form>
  );
}

function SearchKindButton({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      className={`rounded-md px-2 py-1 ${active ? "bg-white text-accent shadow-sm" : "text-muted"}`}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function ModeTabs({
  canisterId,
  selectedPath,
  tab
}: {
  canisterId: string;
  selectedPath: string;
  tab: ModeTab;
}) {
  return (
    <nav className="border-b border-line px-3 py-2" aria-label="Left sidebar mode">
      <div className="grid grid-cols-3 gap-1 rounded-xl border border-line bg-white p-1 text-center text-xs">
        {SIDEBAR_TABS.map((value) => (
          <Link
            key={value}
            href={hrefForPath(canisterId, selectedPath, undefined, value)}
            className={`rounded-lg px-1.5 py-1.5 no-underline ${tab === value ? "bg-accent text-white" : "text-muted hover:bg-paper"}`}
          >
            {value}
          </Link>
        ))}
      </div>
    </nav>
  );
}

function tabTitle(tab: ModeTab): string {
  if (tab === "search") return "Results";
  if (tab === "recent") return "Recent";
  if (tab === "lint") return "Lint Hints";
  return "Explorer";
}

function parseTab(value: string | null): ModeTab {
  if (value === "search" || value === "recent" || value === "lint" || value === "explorer") {
    return value;
  }
  return "explorer";
}

function parseView(value: string | null): ViewMode {
  return value === "raw" ? "raw" : "preview";
}

function parseSearchKind(value: string | null): "path" | "full" {
  return value === "full" ? "full" : "path";
}

function looksLikeFilePath(path: string): boolean {
  const name = path.split("/").filter(Boolean).at(-1) ?? "";
  return /\.[A-Za-z0-9]+$/.test(name);
}

function pathFromSitePathname(canisterId: string, pathname: string): string {
  const prefix = `/site/${encodeURIComponent(canisterId)}/`;
  if (!pathname.startsWith(prefix)) {
    return "/Wiki";
  }
  const suffix = pathname.slice(prefix.length);
  const path = suffix
    .split("/")
    .filter(Boolean)
    .map(decodePathSegment)
    .join("/");
  return path ? `/${path}` : "/Wiki";
}

function decodePathSegment(segment: string): string {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}
