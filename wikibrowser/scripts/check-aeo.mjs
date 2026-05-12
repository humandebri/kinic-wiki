// Where: wikibrowser/scripts/check-aeo.mjs
// What: Static checks for the AEO publish MVP.
// Why: Route generation must stay allowlisted and crawler-safe.

import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const pagesSource = fs.readFileSync(path.join(root, "lib/aeo/pages.ts"), "utf8");
const sitemapSource = fs.readFileSync(path.join(root, "app/sitemap.ts"), "utf8");
const robotsSource = fs.readFileSync(path.join(root, "app/robots.ts"), "utf8");
const configSource = fs.readFileSync(path.join(root, "next.config.ts"), "utf8");
const parserSource = fs.readFileSync(path.join(root, "lib/aeo/parse-markdown.ts"), "utf8");

const failures = [];

for (const slug of [
  "what-is-kinic",
  "what-is-ai-memory",
  "personal-ai-memory",
  "public-memory",
  "read-only-memory",
  "chatgpt-memory-app",
  "kinic-vs-bookmarks",
  "kinic-vs-notion",
  "ai-memory-for-research",
  "ai-memory-for-developers"
]) {
  if (!pagesSource.includes(`"${slug}"`)) {
    failures.push(`missing AEO slug: ${slug}`);
  }
}

if (!pagesSource.includes("KINIC_AEO_CANISTER_ID")) {
  failures.push("AEO canister id is not environment-backed");
}
if (!pagesSource.includes("canonicalPath: `/answers/${slug}`")) {
  failures.push("AEO canonical paths must derive from allowlisted slugs");
}
if (pagesSource.includes("[canisterId]") || pagesSource.includes(":canister")) {
  failures.push("AEO pages must not expose arbitrary canister route params");
}
if (!sitemapSource.includes("listAeoPages")) {
  failures.push("sitemap must use the AEO allowlist");
}
if (sitemapSource.includes("/w/") || sitemapSource.includes("/dashboard/")) {
  failures.push("sitemap must not include browser routes");
}
if (!robotsSource.includes("OAI-SearchBot")) {
  failures.push("robots.txt must include OAI-SearchBot policy");
}
if (!robotsSource.includes('disallow: ["/"]')) {
  failures.push("robots.txt must disallow arbitrary browser routes");
}
if (configSource.includes("output: \"export\"")) {
  failures.push("next.config.ts must not use static export");
}
for (const field of ["title", "description", "answer_summary", "updated", "index", "sources"]) {
  if (!parserSource.includes(field)) {
    failures.push(`frontmatter parser does not handle ${field}`);
  }
}
if (!parserSource.includes("sources: raw.sources ?? []")) {
  failures.push("frontmatter parser must default missing sources to []");
}
if (parserSource.includes("!raw.sources || raw.sources.length === 0")) {
  failures.push("frontmatter parser must not reject legacy AEO Markdown without sources");
}
for (const requiredFragment of [
  "!raw.title",
  "!raw.description",
  "!raw.answer_summary",
  "!raw.updated",
  'raw.index !== "true"'
]) {
  if (!parserSource.includes(requiredFragment)) {
    failures.push(`frontmatter parser must still require ${requiredFragment}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("AEO checks passed");
