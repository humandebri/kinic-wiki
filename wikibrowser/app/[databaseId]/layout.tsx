// Where: wikibrowser/app/[databaseId]/layout.tsx
// What: Persistent database browser shell for every node path in one database.
// Why: App Router pages are route leaves; keeping WikiBrowser in a layout preserves Explorer state across child navigation.

import { Suspense, type ReactNode } from "react";
import { WikiBrowser } from "@/components/wiki-browser";

export default function WikiDatabaseLayout({ children }: { children: ReactNode }) {
  void children;
  return (
    <Suspense fallback={<div className="min-h-screen bg-canvas" />}>
      <WikiBrowser />
    </Suspense>
  );
}
