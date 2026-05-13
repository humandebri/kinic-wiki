// Where: wikibrowser/lib/source-clips.ts
// What: URL clip normalization, extraction, and Markdown rendering helpers.
// Why: Source clips should stay ordinary VFS source nodes while keeping UI logic small.

export const SOURCE_CLIP_PREFIX = "/Sources/raw";

export type SourceClipDraft = {
  url: string;
  title: string;
  site: string;
  capturedAt: string;
  tags: string[];
  userNote: string;
  extractedText: string;
};

export type SourceClipDocument = SourceClipDraft & {
  normalizedUrl: string;
  path: string;
  metadataJson: string;
  markdown: string;
};

export type ExtractedSourceContent = {
  title: string;
  text: string;
};

export async function buildSourceClipDocument(draft: SourceClipDraft): Promise<SourceClipDocument> {
  const normalizedUrl = normalizeClipUrl(draft.url);
  const path = await sourceClipPath(normalizedUrl);
  const title = normalizedTitle(draft.title, normalizedUrl);
  const metadataJson = JSON.stringify({
    app: "source_clip",
    url: normalizedUrl,
    title,
    tags: draft.tags
  });
  return {
    ...draft,
    title,
    normalizedUrl,
    path,
    metadataJson,
    markdown: renderSourceClipMarkdown({
      ...draft,
      title,
      url: normalizedUrl
    })
  };
}

export function normalizeClipUrl(input: string): string {
  const url = new URL(input.trim());
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("URL must use http or https.");
  }
  url.hash = "";
  if ((url.protocol === "https:" && url.port === "443") || (url.protocol === "http:" && url.port === "80")) {
    url.port = "";
  }
  url.hostname = url.hostname.toLowerCase();
  return url.toString();
}

export async function sourceClipPath(normalizedUrl: string): Promise<string> {
  const url = new URL(normalizedUrl);
  const hash = await sha256Prefix(normalizedUrl, 12);
  const id = `clip-${safePathSegment(url.hostname).slice(0, 48)}-${hash}`;
  return `${SOURCE_CLIP_PREFIX}/${id}/${id}.md`;
}

export function parseTags(input: string): string[] {
  const tags = input
    .split(/[,\s]+/)
    .map((tag) => tag.trim().replace(/^#/, ""))
    .filter(Boolean);
  return [...new Set(tags)].slice(0, 12);
}

export function renderSourceClipMarkdown(draft: SourceClipDraft): string {
  const tags = draft.tags.join(", ");
  const title = normalizedTitle(draft.title, draft.url);
  return [
    "---",
    `source_url: ${yamlScalar(draft.url)}`,
    `title: ${yamlScalar(title)}`,
    `site: ${yamlScalar(draft.site)}`,
    `captured_at: ${yamlScalar(draft.capturedAt)}`,
    `tags: ${yamlScalar(tags)}`,
    `user_note: ${yamlScalar(draft.userNote)}`,
    "---",
    "",
    `# ${title}`,
    "",
    `- URL: ${draft.url}`,
    draft.userNote.trim() ? `- Note: ${draft.userNote.trim()}` : "- Note: ",
    tags ? `- Tags: ${tags}` : "- Tags: ",
    "",
    "## Extracted Text",
    "",
    draft.extractedText.trim()
  ].join("\n");
}

export function extractSourceContentFromHtml(html: string, sourceUrl: string): ExtractedSourceContent {
  const document = new DOMParser().parseFromString(html, "text/html");
  const jsonLd = extractRecipeJsonLd(document);
  if (jsonLd) {
    return jsonLd;
  }
  const container = document.querySelector("article") ?? document.querySelector("main") ?? document.body;
  const title = textOf(document.querySelector("title")) || normalizedTitle("", sourceUrl);
  const text = collapseText(container?.textContent ?? "");
  if (!text) {
    throw new Error("Extracted text is empty.");
  }
  return { title, text };
}

export function sourceClipTitleFromMetadata(metadataJson: string, path: string): string {
  const metadata = parseRecord(metadataJson);
  const title = typeof metadata?.title === "string" ? metadata.title.trim() : "";
  if (title) {
    return title;
  }
  return path.split("/").at(-1)?.replace(/\.md$/, "") ?? path;
}

export function sourceClipTagsFromMetadata(metadataJson: string): string[] {
  const metadata = parseRecord(metadataJson);
  if (!metadata || !Array.isArray(metadata.tags)) {
    return [];
  }
  return metadata.tags.filter((tag) => typeof tag === "string").slice(0, 12);
}

export function sourceClipSiteFromMetadata(metadataJson: string): string {
  const metadata = parseRecord(metadataJson);
  if (metadata && typeof metadata.url === "string") {
    try {
      return new URL(metadata.url).hostname;
    } catch {
      return "";
    }
  }
  return "";
}

function extractRecipeJsonLd(document: Document): ExtractedSourceContent | null {
  for (const script of document.querySelectorAll('script[type="application/ld+json"]')) {
    const parsed = parseJsonValue(script.textContent ?? "");
    const recipes = collectRecipeObjects(parsed);
    const recipe = recipes[0];
    if (!recipe) {
      continue;
    }
    const title = stringValue(recipe.name);
    const sections = [
      stringValue(recipe.description),
      listSection("Ingredients", recipe.recipeIngredient),
      instructionsSection(recipe.recipeInstructions)
    ].filter(Boolean);
    const text = collapseText(sections.join("\n\n"));
    if (title || text) {
      return { title: title || "Recipe", text };
    }
  }
  return null;
}

function collectRecipeObjects(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value.flatMap(collectRecipeObjects);
  }
  if (!isRecord(value)) {
    return [];
  }
  const graph = value["@graph"];
  const nested = Array.isArray(graph) ? graph.flatMap(collectRecipeObjects) : [];
  return isRecipeObject(value) ? [value, ...nested] : nested;
}

function isRecipeObject(value: Record<string, unknown>): boolean {
  const type = value["@type"];
  if (typeof type === "string") {
    return type.toLowerCase() === "recipe";
  }
  return Array.isArray(type) && type.some((item) => typeof item === "string" && item.toLowerCase() === "recipe");
}

function listSection(title: string, value: unknown): string {
  const items = stringList(value);
  return items.length > 0 ? `${title}\n${items.map((item) => `- ${item}`).join("\n")}` : "";
}

function instructionsSection(value: unknown): string {
  const items = instructionList(value);
  return items.length > 0 ? `Instructions\n${items.map((item, index) => `${index + 1}. ${item}`).join("\n")}` : "";
}

function instructionList(value: unknown): string[] {
  if (typeof value === "string") {
    return [value.trim()].filter(Boolean);
  }
  if (Array.isArray(value)) {
    return value.flatMap(instructionList);
  }
  if (isRecord(value)) {
    const text = stringValue(value.text);
    if (text) {
      return [text];
    }
    return instructionList(value.itemListElement);
  }
  return [];
}

function stringList(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map(stringValue).filter(Boolean);
  }
  const single = stringValue(value);
  return single ? [single] : [];
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? collapseText(value) : "";
}

function normalizedTitle(title: string, url: string): string {
  const trimmed = title.trim();
  if (trimmed) {
    return trimmed;
  }
  try {
    return new URL(url).hostname;
  } catch {
    return "Untitled source";
  }
}

function safePathSegment(value: string): string {
  const normalized = value.toLowerCase().replace(/[^a-z0-9.-]+/g, "-").replace(/^-+|-+$/g, "");
  return normalized || "site";
}

function yamlScalar(value: string): string {
  return JSON.stringify(value);
}

function textOf(element: Element | null): string {
  return collapseText(element?.textContent ?? "");
}

function collapseText(value: string): string {
  return value.replace(/\r/g, "").replace(/[ \t]+\n/g, "\n").replace(/\n{3,}/g, "\n\n").trim();
}

function parseRecord(value: string): Record<string, unknown> | null {
  const parsed = parseJsonValue(value);
  return isRecord(parsed) ? parsed : null;
}

function parseJsonValue(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

async function sha256Prefix(value: string, length: number): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")
    .slice(0, length);
}
