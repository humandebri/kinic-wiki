import { WikiBrowser } from "@/components/wiki-browser";
import { pathFromSegments } from "@/lib/paths";
import type { ModeTab } from "@/lib/wiki-helpers";

type PageProps = {
  params: Promise<{
    canisterId: string;
    nodePath?: string[];
  }>;
  searchParams: Promise<{
    view?: string;
    tab?: string;
    q?: string;
  }>;
};

export default async function SitePage({ params, searchParams }: PageProps) {
  const resolvedParams = await params;
  const resolvedSearch = await searchParams;
  const selectedPath = pathFromSegments(resolvedParams.nodePath ?? []);
  const initialView = resolvedSearch.view === "raw" ? "raw" : "preview";
  const initialTab = parseTab(resolvedSearch.tab);

  return (
    <WikiBrowser
      canisterId={resolvedParams.canisterId}
      selectedPath={selectedPath}
      initialView={initialView}
      initialTab={initialTab}
      initialQuery={resolvedSearch.q ?? ""}
    />
  );
}

function parseTab(tab: string | undefined): ModeTab {
  if (tab === "search" || tab === "recent" || tab === "lint") {
    return tab;
  }
  return "explorer";
}
