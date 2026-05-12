// Where: wikibrowser/lib/aeo/site.ts
// What: Public URL helpers for metadata, sitemap, robots, and llms.txt.
// Why: Canonical generation must be centralized across server routes.

const LOCAL_SITE_URL = "http://localhost:3000";

export function siteUrl(): string {
  const explicitUrl = process.env.NEXT_PUBLIC_SITE_URL;
  if (explicitUrl) {
    return normalizeOrigin(explicitUrl);
  }
  const vercelUrl = process.env.VERCEL_URL;
  if (vercelUrl) {
    return normalizeOrigin(`https://${vercelUrl}`);
  }
  return LOCAL_SITE_URL;
}

export function absoluteUrl(path: string): string {
  return new URL(path, siteUrl()).toString();
}

function normalizeOrigin(origin: string): string {
  return origin.endsWith("/") ? origin.slice(0, -1) : origin;
}
