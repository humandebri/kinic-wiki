"use client";

import type { FormEvent } from "react";
import { useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { GitBranch, Network, PanelRight, Search } from "lucide-react";
import { CycleBattery } from "@/components/cycle-battery";
import { DocumentHeader, DocumentPane } from "@/components/document-pane";
import { ExplorerTree } from "@/components/explorer-tree";
import { Inspector } from "@/components/inspector";
import { LintPanel } from "@/components/lint-panel";
import { PanelHeader } from "@/components/panel";
import { RecentPanel } from "@/components/recent-panel";
import { hrefForGraph, hrefForPath, hrefForSearch } from "@/lib/paths";
import { nodeRequestKey } from "@/lib/request-keys";
import type { ChildNode, NodeContext, WikiNode } from "@/lib/types";
import {
  errorHint,
  errorMessage,
  inferNoteRole,
  isNotFoundError,
  loadingState,
  ApiError,
  type ModeTab,
  type PathLoadState,
  type ViewMode
} from "@/lib/wiki-helpers";

const SIDEBAR_TABS: ModeTab[] = ["explorer", "recent", "lint"];
const GraphPanel = dynamic(() => import("@/components/graph-panel").then((module) => module.GraphPanel), {
  ssr: false,
  loading: () => <p className="min-h-0 flex-1 p-5 text-sm text-muted">Loading graph view...</p>
});
const SearchPanel = dynamic(() => import("@/components/search-panel").then((module) => module.SearchPanel), {
  ssr: false,
  loading: () => <p className="min-h-0 flex-1 p-5 text-sm text-muted">Loading search...</p>
});

type BrowserLoadState<T> = PathLoadState<T> & {
  requestKey: string;
};

export function WikiBrowser() {
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const routeState = useMemo(() => parseWikiRoute(pathname), [pathname]);
  const canisterId = routeState.canisterId ?? "";
  const isSearchPage = useMemo(() => isBrowserSearchPathname(canisterId, pathname), [canisterId, pathname]);
  const isGraphPage = useMemo(() => isBrowserGraphPathname(canisterId, pathname), [canisterId, pathname]);
  const graphCenter = isGraphPage ? searchParams.get("center") : null;
  const graphDepth = parseGraphDepth(searchParams.get("depth"));
  const selectedPath = useMemo(
    () => isSearchPage ? "/Wiki" : isGraphPage ? graphCenter ?? "/Wiki" : routeState.nodePath,
    [graphCenter, isGraphPage, isSearchPage, routeState.nodePath]
  );
  const view = parseView(searchParams.get("view"));
  const tab = parseTab(searchParams.get("tab"));
  const query = isSearchPage ? searchParams.get("q") ?? "" : "";
  const searchKind = parseSearchKind(searchParams.get("kind"));
  const currentRequestKey = nodeRequestKey(canisterId, selectedPath);
  const [node, setNode] = useState<BrowserLoadState<WikiNode>>(browserLoadingState(canisterId, selectedPath));
  const [nodeContext, setNodeContext] = useState<BrowserLoadState<NodeContext>>(browserLoadingState(canisterId, selectedPath));
  const [childNodes, setChildNodes] = useState<BrowserLoadState<ChildNode[]>>(browserLoadingState(canisterId, selectedPath));
  const invalidCanister = validateCanisterText(canisterId);

  useEffect(() => {
    let cancelled = false;
    if (typeof invalidCanister === "string") {
      return;
    }
    if (isGraphPage && !graphCenter) {
      return;
    }
    const requestKey = nodeRequestKey(canisterId, selectedPath);
    import("@/lib/vfs-client")
      .then(({ readNodeContext }) => readNodeContext(canisterId, selectedPath, 20))
      .then((data) => {
        if (!cancelled) {
          if (!data) {
            throw new ApiError(`node not found: ${selectedPath}`, 404);
          }
          setNode({ requestKey, path: selectedPath, data: data.node, error: null, loading: false });
          setNodeContext({ requestKey, path: selectedPath, data, error: null, loading: false });
          setChildNodes({ requestKey, path: selectedPath, data: [], error: null, loading: false });
        }
      })
      .catch((nodeError: Error) => {
        if (!isNotFoundError(nodeError)) {
          if (!cancelled) {
            setNode({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
            setNodeContext({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
            setChildNodes({ requestKey, path: selectedPath, data: null, error: null, loading: false });
          }
          return;
        }
        import("@/lib/vfs-client")
          .then(({ listChildren }) => listChildren(canisterId, selectedPath))
          .then((data) => {
            if (!cancelled) {
              if (data.length === 0 && looksLikeFilePath(selectedPath)) {
                setNode({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
                setNodeContext({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
                setChildNodes({ requestKey, path: selectedPath, data: null, error: `path not found: ${selectedPath}`, loading: false });
              } else {
                setNode({ requestKey, path: selectedPath, data: null, error: null, loading: false });
                setNodeContext({ requestKey, path: selectedPath, data: null, error: null, loading: false });
                setChildNodes({ requestKey, path: selectedPath, data, error: null, loading: false });
              }
            }
          })
          .catch((childrenError: Error) => {
            if (!cancelled) {
              setNode({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
              setNodeContext({ requestKey, path: selectedPath, data: null, error: errorMessage(nodeError), hint: errorHint(nodeError), loading: false });
              setChildNodes({ requestKey, path: selectedPath, data: null, error: errorMessage(childrenError), hint: errorHint(childrenError), loading: false });
            }
          });
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId, graphCenter, invalidCanister, isGraphPage, selectedPath]);

  const currentNode = currentNodeState(invalidCanister, canisterId, selectedPath, currentRequestKey, node);
  const currentNodeContext = currentNodeContextState(invalidCanister, canisterId, selectedPath, currentRequestKey, nodeContext);
  const currentChildren = currentChildrenState(invalidCanister, canisterId, selectedPath, currentRequestKey, childNodes);
  const noteRole = inferNoteRole(selectedPath);

  return (
    <main className="flex h-screen flex-col overflow-hidden bg-canvas text-ink">
      <TopBar
        canisterId={canisterId}
        query={query}
        searchKind={searchKind}
        isGraphPage={isGraphPage}
        graphCenter={graphCenter}
        selectedPath={selectedPath}
      />
      <section className={`grid min-h-0 flex-1 grid-cols-1 gap-3 p-3 ${isSearchPage || isGraphPage ? "lg:grid-cols-[320px_minmax(0,1fr)]" : "lg:grid-cols-[320px_minmax(0,1fr)_320px]"}`}>
        <aside className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
          <PanelHeader icon={<GitBranch size={15} />} title={tabTitle(tab)} />
          <ModeTabs canisterId={canisterId} selectedPath={selectedPath} tab={tab} />
          <LeftPane
            tab={tab}
            canisterId={canisterId}
            selectedPath={selectedPath}
            node={currentNode.data}
            autoExpandExplorer={!(isGraphPage && !graphCenter)}
          />
        </aside>
        <section className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
          {isGraphPage ? (
            <GraphPanel canisterId={canisterId} centerPath={graphCenter} depth={graphDepth} />
          ) : isSearchPage ? (
            <SearchPanel canisterId={canisterId} query={query} initialKind={searchKind} />
          ) : (
            <>
              <DocumentHeader
                path={selectedPath}
                view={view}
                onViewChange={(nextView) => {
                  router.replace(hrefForPath(canisterId, selectedPath, nextView, tab));
                }}
                isDirectory={!currentNode.data && Boolean(currentChildren.data)}
              />
              <DocumentPane
                node={currentNode}
                childrenState={currentChildren}
                view={view}
                canisterId={canisterId}
              />
            </>
          )}
        </section>
        {!isSearchPage && !isGraphPage ? (
          <aside className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-line bg-paper/90 shadow-sm">
            <PanelHeader icon={<PanelRight size={15} />} title="Inspector" subtitle="metadata and hints" />
            <Inspector
              canisterId={canisterId}
              path={selectedPath}
              node={currentNode.data}
              childNodes={currentChildren.data ?? []}
              noteRole={noteRole}
              incomingLinks={currentNodeContext.data?.incomingLinks ?? null}
              incomingError={currentNodeContext.error}
              outgoingLinks={currentNodeContext.data?.outgoingLinks ?? []}
            />
          </aside>
        ) : null}
      </section>
    </main>
  );
}

function LeftPane({
  tab,
  canisterId,
  selectedPath,
  node,
  autoExpandExplorer
}: {
  tab: ModeTab;
  canisterId: string;
  selectedPath: string;
  node: WikiNode | null;
  autoExpandExplorer: boolean;
}) {
  if (tab === "recent") return <RecentPanel canisterId={canisterId} />;
  if (tab === "lint") return <LintPanel path={selectedPath} node={node} canisterId={canisterId} />;
  return <ExplorerTree canisterId={canisterId} selectedPath={selectedPath} autoExpandSelected={autoExpandExplorer} />;
}

function TopBar({
  canisterId,
  query,
  searchKind,
  isGraphPage,
  graphCenter,
  selectedPath
}: {
  canisterId: string;
  query: string;
  searchKind: "path" | "full";
  isGraphPage: boolean;
  graphCenter: string | null;
  selectedPath: string;
}) {
  const graphLinkCenter = isGraphPage ? graphCenter : selectedPath;
  return (
    <header className="flex min-h-[52px] items-center gap-4 border-b border-line bg-paper/80 px-3 py-2 backdrop-blur">
      <div className="w-[168px] shrink-0">
        <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted">Wiki Canister Browser</p>
        <h1 className="text-base font-semibold leading-tight tracking-[-0.03em]">Knowledge IDE</h1>
      </div>
      <div className="flex min-w-0 flex-1 items-center justify-end gap-2">
        <Link
          className={`hidden items-center gap-1 rounded-lg border border-line px-3 py-2 text-sm no-underline md:flex ${isGraphPage ? "bg-accent text-white" : "bg-white text-ink"}`}
          href={hrefForGraph(canisterId, graphLinkCenter)}
        >
          <Network size={15} />
          Graph
        </Link>
        <CycleBattery canisterId={canisterId} />
        <HeaderSearch canisterId={canisterId} query={query} searchKind={searchKind} />
      </div>
    </header>
  );
}

function HeaderSearch({
  canisterId,
  query,
  searchKind
}: {
  canisterId: string;
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
    router.replace(hrefForSearch(canisterId, text.trim(), kind));
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
  if (tab === "recent") return "Recent";
  if (tab === "lint") return "Lint Hints";
  return "Explorer";
}

function parseTab(value: string | null): ModeTab {
  if (value === "recent" || value === "lint" || value === "explorer") {
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

function parseGraphDepth(value: string | null): 1 | 2 {
  return value === "2" ? 2 : 1;
}

function currentNodeState(
  invalidCanister: string | null,
  canisterId: string,
  selectedPath: string,
  requestKey: string,
  node: BrowserLoadState<WikiNode>
): PathLoadState<WikiNode> {
  if (typeof invalidCanister === "string") {
    return { path: selectedPath, data: null, error: "Invalid canister ID", hint: invalidCanister, loading: false };
  }
  return node.requestKey === requestKey ? node : browserLoadingState<WikiNode>(canisterId, selectedPath);
}

function currentNodeContextState(
  invalidCanister: string | null,
  canisterId: string,
  selectedPath: string,
  requestKey: string,
  nodeContext: BrowserLoadState<NodeContext>
): PathLoadState<NodeContext> {
  if (typeof invalidCanister === "string") {
    return { path: selectedPath, data: null, error: "Invalid canister ID", hint: invalidCanister, loading: false };
  }
  return nodeContext.requestKey === requestKey ? nodeContext : browserLoadingState<NodeContext>(canisterId, selectedPath);
}

function currentChildrenState(
  invalidCanister: string | null,
  canisterId: string,
  selectedPath: string,
  requestKey: string,
  childNodes: BrowserLoadState<ChildNode[]>
): PathLoadState<ChildNode[]> {
  if (typeof invalidCanister === "string") {
    return { path: selectedPath, data: null, error: null, loading: false };
  }
  return childNodes.requestKey === requestKey ? childNodes : browserLoadingState<ChildNode[]>(canisterId, selectedPath);
}

function browserLoadingState<T>(canisterId: string, path: string): BrowserLoadState<T> {
  return { ...loadingState<T>(path), requestKey: nodeRequestKey(canisterId, path) };
}

function looksLikeFilePath(path: string): boolean {
  const name = path.split("/").filter(Boolean).at(-1) ?? "";
  return /\.[A-Za-z0-9]+$/.test(name);
}

function validateCanisterText(canisterId: string): string | null {
  if (!canisterId) {
    return "missing canister id";
  }
  if (!/^[a-z0-9-]+$/i.test(canisterId)) {
    return "canister id contains unsupported characters";
  }
  return null;
}

function parseWikiRoute(pathname: string): { canisterId: string | null; nodePath: string } {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "w" || !segments[1]) {
    return { canisterId: null, nodePath: "/Wiki" };
  }
  const path = segments
    .slice(2)
    .filter(Boolean)
    .map(decodePathSegment)
    .join("/");
  return {
    canisterId: decodePathSegment(segments[1]),
    nodePath: path ? `/${path}` : "/Wiki"
  };
}

function isBrowserSearchPathname(canisterId: string, pathname: string): boolean {
  return pathname === `/w/${encodeURIComponent(canisterId)}/search`;
}

function isBrowserGraphPathname(canisterId: string, pathname: string): boolean {
  return pathname === `/w/${encodeURIComponent(canisterId)}/graph`;
}

function decodePathSegment(segment: string): string {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}
