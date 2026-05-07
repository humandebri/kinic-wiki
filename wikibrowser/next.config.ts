import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  reactStrictMode: true,
  async headers() {
    return [
      {
        source: "/w/:segments*",
        headers: [
          {
            key: "X-Robots-Tag",
            value: "noindex, nofollow"
          }
        ]
      }
    ];
  },
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
