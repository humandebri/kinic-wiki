import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  output: "export",
  env: {
    KINIC_WIKI_CANISTER_ID: process.env.KINIC_WIKI_CANISTER_ID ?? ""
  },
  reactStrictMode: true,
  async rewrites() {
    return [
      {
        source: "/:databaseId",
        destination: "/w"
      },
      {
        source: "/:databaseId/:segments*",
        destination: "/w"
      }
    ];
  }
};

export default nextConfig;
