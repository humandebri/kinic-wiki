// Where: wikibrowser/lib/aeo/pages.ts
// What: Allowlisted answer page targets for AI-readable public memory pages.
// Why: Indexed pages must never expose arbitrary canister IDs or VFS paths.

export type AeoPageConfig = {
  slug: string;
  canisterId: string;
  databaseId: string;
  path: string;
  canonicalPath: string;
  locale: string;
};

type AeoPageSeed = {
  path: string;
  title: string;
};

const DEFAULT_DATABASE_ID = "default";
const DEFAULT_LOCALE = "en";
const aeoCanisterId = process.env.KINIC_AEO_CANISTER_ID ?? "";

const PAGE_SEEDS: Record<string, AeoPageSeed> = {
  "what-is-kinic": {
    path: "/Wiki/aeo/what-is-kinic.md",
    title: "What is Kinic?"
  },
  "what-is-ai-memory": {
    path: "/Wiki/aeo/what-is-ai-memory.md",
    title: "What is AI memory?"
  },
  "personal-ai-memory": {
    path: "/Wiki/aeo/personal-ai-memory.md",
    title: "What is personal AI memory?"
  },
  "public-memory": {
    path: "/Wiki/aeo/public-memory.md",
    title: "What is public memory?"
  },
  "read-only-memory": {
    path: "/Wiki/aeo/read-only-memory.md",
    title: "What is read-only memory?"
  },
  "chatgpt-memory-app": {
    path: "/Wiki/aeo/chatgpt-memory-app.md",
    title: "What is a ChatGPT memory app?"
  },
  "kinic-vs-bookmarks": {
    path: "/Wiki/comparisons/kinic-vs-bookmarks.md",
    title: "Kinic vs bookmarks"
  },
  "kinic-vs-notion": {
    path: "/Wiki/comparisons/kinic-vs-notion.md",
    title: "Kinic vs Notion"
  },
  "ai-memory-for-research": {
    path: "/Wiki/aeo/ai-memory-for-research.md",
    title: "AI memory for research"
  },
  "ai-memory-for-developers": {
    path: "/Wiki/aeo/ai-memory-for-developers.md",
    title: "AI memory for developers"
  }
};

export const AEO_PAGES: Record<string, AeoPageConfig> = Object.fromEntries(
  Object.entries(PAGE_SEEDS).map(([slug, seed]) => [
    slug,
    {
      slug,
      canisterId: aeoCanisterId,
      databaseId: DEFAULT_DATABASE_ID,
      path: seed.path,
      canonicalPath: `/answers/${slug}`,
      locale: DEFAULT_LOCALE
    }
  ])
);

export function getAeoPage(slug: string): AeoPageConfig | null {
  return AEO_PAGES[slug] ?? null;
}

export function listAeoPages(): AeoPageConfig[] {
  return Object.values(AEO_PAGES);
}

export function listAeoPageLinks(): { title: string; path: string }[] {
  return Object.entries(PAGE_SEEDS).map(([slug, seed]) => ({
    title: seed.title,
    path: `/answers/${slug}`
  }));
}
