import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Wiki Canister Browser",
  description: "Read-only browser for Kinic Wiki canisters"
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
