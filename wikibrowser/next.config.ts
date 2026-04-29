import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  output: "export",
  reactStrictMode: true,
  async rewrites() {
    return [
      {
        source: "/w/:segments*",
        destination: "/w"
      }
    ];
  }
};

export default nextConfig;
