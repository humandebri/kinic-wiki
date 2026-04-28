import type { ReactNode } from "react";
import { WikiBrowser } from "@/components/wiki-browser";

type WikiLayoutProps = {
  children: ReactNode;
  params: Promise<{
    canisterId: string;
  }>;
};

export default async function WikiLayout({ children, params }: WikiLayoutProps) {
  const { canisterId } = await params;
  return (
    <>
      <WikiBrowser canisterId={canisterId} />
      {children}
    </>
  );
}
