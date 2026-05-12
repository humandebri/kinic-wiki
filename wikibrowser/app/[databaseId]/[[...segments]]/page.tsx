import { Suspense } from "react";
import type { Metadata } from "next";
import { WikiBrowser } from "@/components/wiki-browser";

export const metadata: Metadata = {
  robots: {
    index: false,
    follow: false
  }
};

export default function WikiDatabasePage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-canvas" />}>
      <WikiBrowser />
    </Suspense>
  );
}
