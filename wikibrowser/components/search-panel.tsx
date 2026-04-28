"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { FileSearch, Search } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { SearchNodeHit } from "@/lib/types";
import { apiPath, fetchJson } from "@/lib/wiki-helpers";
import { ErrorBox } from "@/components/panel";

type SearchKind = "path" | "full";

export function SearchPanel({
  canisterId,
  selectedPath,
  query
}: {
  canisterId: string;
  selectedPath: string;
  query: string;
}) {
  const router = useRouter();
  const initialSearch = useRef({
    canisterId,
    selectedPath,
    query: query.trim()
  });
  const didRunInitialSearch = useRef(false);
  const [text, setText] = useState(query);
  const [kind, setKind] = useState<SearchKind>("path");
  const [results, setResults] = useState<SearchNodeHit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(Boolean(query.trim()));

  useEffect(() => {
    if (didRunInitialSearch.current) return;
    const initialQuery = initialSearch.current.query;
    if (!initialQuery) return;
    didRunInitialSearch.current = true;
    let cancelled = false;
    const params = new URLSearchParams({
      q: initialQuery,
      limit: "20",
      prefix: rootPrefix(initialSearch.current.selectedPath)
    });
    fetchJson<SearchNodeHit[]>(apiPath(initialSearch.current.canisterId, "search-path", params))
      .then((data) => {
        if (!cancelled) setResults(data);
      })
      .catch((searchError: Error) => {
        if (!cancelled) setError(searchError.message);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function runSearchText(searchText: string, updateUrl: boolean, searchKind: SearchKind) {
    if (!searchText) {
      setResults([]);
      setError("missing q");
      if (updateUrl) {
        router.replace(hrefForPath(canisterId, selectedPath, undefined, "search"));
      }
      return;
    }
    const endpoint = searchKind === "path" ? "search-path" : "search";
    const params = new URLSearchParams({ q: searchText, limit: "20", prefix: rootPrefix(selectedPath) });
    if (updateUrl) {
      router.replace(hrefForPath(canisterId, selectedPath, undefined, "search", searchText));
    }
    setLoading(true);
    setError(null);
    fetchJson<SearchNodeHit[]>(apiPath(canisterId, endpoint, params))
      .then((data) => setResults(data))
      .catch((searchError: Error) => setError(searchError.message))
      .finally(() => setLoading(false));
  }

  function runSearch() {
    runSearchText(text.trim(), true, kind);
  }

  return (
    <div className="space-y-3 overflow-auto p-3">
      <div className="space-y-2 rounded-xl border border-line bg-white p-3">
        <div className="flex rounded-lg border border-line bg-paper p-1 text-xs">
          <KindButton active={kind === "path"} label="Path" onClick={() => setKind("path")} />
          <KindButton active={kind === "full"} label="Full text" onClick={() => setKind("full")} />
        </div>
        <input
          className="w-full rounded-lg border border-line px-3 py-2 text-sm outline-none focus:border-accent"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") runSearch();
          }}
          placeholder="Search wiki"
        />
        <button
          className="flex w-full items-center justify-center gap-2 rounded-lg bg-accent px-3 py-2 text-sm text-white"
          type="button"
          onClick={runSearch}
        >
          {kind === "path" ? <FileSearch size={14} /> : <Search size={14} />}
          {loading ? "Searching" : "Search"}
        </button>
      </div>
      {error ? <ErrorBox message={error} /> : null}
      <div className="space-y-2">
        {results.map((hit) => (
          <Link
            key={`${hit.path}-${hit.score}`}
            href={hrefForPath(canisterId, hit.path)}
            className="block rounded-xl border border-line bg-white p-3 text-sm no-underline hover:border-accent"
          >
            <div className="truncate font-mono text-xs text-accent">{hit.path}</div>
            <div className="mt-1 text-xs text-muted">{hit.matchReasons.join(", ") || hit.kind}</div>
            {hit.preview?.excerpt || hit.snippet ? (
              <p className="mt-2 line-clamp-3 text-xs text-ink">{hit.preview?.excerpt ?? hit.snippet}</p>
            ) : null}
          </Link>
        ))}
      </div>
    </div>
  );
}

function KindButton({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button type="button" className={`flex-1 rounded-md px-2 py-1 ${active ? "bg-white text-accent" : "text-muted"}`} onClick={onClick}>
      {label}
    </button>
  );
}

function rootPrefix(path: string): string {
  return path.startsWith("/Sources") ? "/Sources" : "/Wiki";
}
