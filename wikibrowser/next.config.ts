import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  env: {
    KINIC_WIKI_CANISTER_ID: process.env.KINIC_WIKI_CANISTER_ID ?? ""
  },
  reactStrictMode: true
};

export default nextConfig;
