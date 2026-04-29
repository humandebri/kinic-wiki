"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Clock } from "lucide-react";
import { hrefForPath } from "@/lib/paths";
import type { RecentNode } from "@/lib/types";
import { errorHint, errorMessage, type LoadState } from "@/lib/wiki-helpers";
import { ErrorBox } from "@/components/panel";

export function RecentPanel({ canisterId }: { canisterId: string }) {
  const [recent, setRecent] = useState<LoadState<RecentNode[]>>({
    data: null,
    error: null,
    loading: true
  });

  useEffect(() => {
    let cancelled = false;
    import("@/lib/vfs-client")
      .then(({ recentNodes }) => recentNodes(canisterId, 30))
      .then((data) => {
        if (!cancelled) setRecent({ data, error: null, loading: false });
      })
      .catch((error: Error) => {
        if (!cancelled) setRecent({ data: null, error: errorMessage(error), hint: errorHint(error), loading: false });
      });
    return () => {
      cancelled = true;
    };
  }, [canisterId]);

  if (recent.error) return <div className="min-h-0 flex-1 p-3"><ErrorBox message={recent.error} hint={recent.hint} /></div>;
  if (recent.loading) return <p className="min-h-0 flex-1 p-4 text-sm text-muted">Loading recent nodes</p>;
  return (
    <div className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
      {recent.data?.map((node) => (
        <Link
          key={node.path}
          href={hrefForPath(canisterId, node.path)}
          className="block rounded-xl border border-line bg-white p-3 text-sm no-underline hover:border-accent"
        >
          <div className="flex items-center gap-2">
            <Clock size={14} className="text-muted" />
            <span className="truncate font-mono text-xs text-accent">{node.path}</span>
          </div>
          <div className="mt-2 flex justify-between font-mono text-[11px] text-muted">
            <span>{node.kind}</span>
            <span>{node.updatedAt}</span>
          </div>
        </Link>
      ))}
    </div>
  );
}
