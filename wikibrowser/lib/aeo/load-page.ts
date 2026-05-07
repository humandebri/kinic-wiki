// Where: wikibrowser/lib/aeo/load-page.ts
// What: Load allowlisted answer pages from a Kinic Wiki canister.
// Why: AEO pages must embed canister-backed memory in server-rendered HTML.

import { readNode } from "@/lib/vfs-client";
import { getAeoPage, type AeoPageConfig } from "@/lib/aeo/pages";
import { parseAeoMarkdown, type ParsedAeoMarkdown } from "@/lib/aeo/parse-markdown";

export type LoadedAeoPage = {
  config: AeoPageConfig;
  parsed: ParsedAeoMarkdown;
};

export async function loadAeoPage(slug: string): Promise<LoadedAeoPage | null> {
  const config = getAeoPage(slug);
  if (!config || !config.canisterId) {
    return null;
  }
  const node = await readNode(config.canisterId, config.databaseId, config.path);
  if (!node || node.kind !== "file") {
    return null;
  }
  const parsed = parseAeoMarkdown(node.content);
  if (!parsed) {
    return null;
  }
  return { config, parsed };
}
