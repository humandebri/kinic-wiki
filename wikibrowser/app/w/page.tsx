import { Suspense } from "react";
import { WikiBrowser } from "@/components/wiki-browser";

export default function WikiPage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-canvas" />}>
      <WikiBrowser />
    </Suspense>
  );
}
