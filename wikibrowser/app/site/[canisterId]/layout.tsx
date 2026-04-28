import type { ReactNode } from "react";
import { WikiBrowser } from "@/components/wiki-browser";

type SiteLayoutProps = {
  children: ReactNode;
  params: Promise<{
    canisterId: string;
  }>;
};

export default async function SiteLayout({ children, params }: SiteLayoutProps) {
  const { canisterId } = await params;
  return (
    <>
      <WikiBrowser canisterId={canisterId} />
      {children}
    </>
  );
}
