"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
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
  extractMarkdownLinks,
  fetchJson,
  inferNoteRole,
  loadingState,
  type ModeTab,
  type PathLoadState,
  type ViewMode
} from "@/lib/wiki-helpers";

const MODE_TABS: ModeTab[] = ["explorer", "search", "recent", "lint"];

type WikiBrowserProps = {
  canisterId: string;
  selectedPath: string;
  initialView: ViewMode;
  initialTab: ModeTab;
  initialQuery: string;
};

export function WikiBrowser({
  canisterId,
  selectedPath,
  initialView,
  initialTab,
  initialQuery
}: WikiBrowserProps) {
  const searchParams = useSearchParams();
  const [view, setView] = useState<ViewMode>(initialView);
  const tab = parseTab(searchParams.get("tab"), initialTab);
  const query = searchParams.get("q") ?? initialQuery;
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
        fetchJson<ChildNode[]>(childrenUrl)
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
  const currentChildren = childNodes.path === selectedPath ? childNodes : loadingState<ChildNode[]>(selectedPath);
  const outgoingLinks = useMemo(() => extractMarkdownLinks(currentNode.data?.content ?? ""), [currentNode.data]);
  const noteRole = inferNoteRole(selectedPath);

  return (
    <main className="flex min-h-screen flex-col bg-canvas text-ink">
      <TopBar canisterId={canisterId} selectedPath={selectedPath} tab={tab} />
      <section className="grid min-h-0 flex-1 grid-cols-1 gap-3 p-3 lg:grid-cols-[320px_minmax(0,1fr)_320px]">
        <aside className="min-h-[280px] overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<GitBranch size={15} />} title={tabTitle(tab)} subtitle="read-only tools" />
          <LeftPane
            tab={tab}
            canisterId={canisterId}
            selectedPath={selectedPath}
            query={query}
            node={currentNode.data}
          />
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
          <PanelHeader icon={<PanelRight size={15} />} title="Inspector" subtitle="metadata and hints" />
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

function LeftPane({
  tab,
  canisterId,
  selectedPath,
  query,
  node
}: {
  tab: ModeTab;
  canisterId: string;
  selectedPath: string;
  query: string;
  node: WikiNode | null;
}) {
  if (tab === "search") return <SearchPanel canisterId={canisterId} selectedPath={selectedPath} query={query} />;
  if (tab === "recent") return <RecentPanel canisterId={canisterId} />;
  if (tab === "lint") return <LintPanel path={selectedPath} node={node} />;
  return <ExplorerTree canisterId={canisterId} selectedPath={selectedPath} />;
}

function TopBar({
  canisterId,
  selectedPath,
  tab
}: {
  canisterId: string;
  selectedPath: string;
  tab: ModeTab;
}) {
  return (
    <header className="flex flex-col gap-3 border-b border-line bg-paper/80 px-4 py-3 backdrop-blur md:flex-row md:items-center md:justify-between">
      <div>
        <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-muted">Wiki Canister Browser</p>
        <h1 className="mt-1 text-lg font-semibold tracking-[-0.03em]">Knowledge IDE</h1>
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-2 md:max-w-3xl">
        <div className="flex flex-wrap gap-1 rounded-xl border border-line bg-white p-1 text-sm">
          {MODE_TABS.map((value) => (
            <Link
              key={value}
              href={hrefForPath(canisterId, selectedPath, undefined, value)}
              className={`rounded-lg px-3 py-1.5 no-underline ${tab === value ? "bg-accent text-white" : "text-muted"}`}
            >
              {value}
            </Link>
          ))}
        </div>
        <div className="flex min-w-0 items-center gap-2 rounded-xl border border-line bg-white px-3 py-2 text-sm text-muted">
          <Search size={15} />
          <span className="truncate">{canisterId} / {selectedPath}</span>
        </div>
      </div>
    </header>
  );
}

function tabTitle(tab: ModeTab): string {
  if (tab === "search") return "Search";
  if (tab === "recent") return "Recent";
  if (tab === "lint") return "Lint Hints";
  return "Explorer";
}

function parseTab(value: string | null, fallback: ModeTab): ModeTab {
  if (value === "search" || value === "recent" || value === "lint" || value === "explorer") {
    return value;
  }
  return fallback;
}
