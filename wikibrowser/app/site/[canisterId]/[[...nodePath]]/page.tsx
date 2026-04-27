import { WikiBrowser } from "@/components/wiki-browser";
import { pathFromSegments } from "@/lib/paths";

type PageProps = {
  params: Promise<{
    canisterId: string;
    nodePath?: string[];
  }>;
  searchParams: Promise<{
    view?: string;
    tab?: string;
  }>;
};

export default async function SitePage({ params, searchParams }: PageProps) {
  const resolvedParams = await params;
  const resolvedSearch = await searchParams;
  const selectedPath = pathFromSegments(resolvedParams.nodePath ?? []);
  const initialView = resolvedSearch.view === "raw" ? "raw" : "preview";

  return (
    <WikiBrowser
      canisterId={resolvedParams.canisterId}
      selectedPath={selectedPath}
      initialView={initialView}
    />
  );
}
