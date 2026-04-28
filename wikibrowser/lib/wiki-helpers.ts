import type { ChildNode } from "@/lib/types";

export type ViewMode = "preview" | "raw";
export type ModeTab = "explorer" | "search" | "recent" | "lint";

export type LoadState<T> = { data: T | null; error: string | null; loading: boolean };
export type PathLoadState<T> = LoadState<T> & { path: string };

export function rootChild(path: "/Wiki" | "/Sources"): ChildNode {
  return {
    path,
    name: path.slice(1),
    kind: "directory",
    updatedAt: null,
    etag: null,
    sizeBytes: null,
    isVirtual: true
  };
}

export function apiPath(canisterId: string, endpoint: string, params: URLSearchParams): string {
  return `/api/site/${encodeURIComponent(canisterId)}/${endpoint}?${params.toString()}`;
}

export async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  const body: unknown = await response.json();
  if (!response.ok) {
    throw new Error(isErrorBody(body) ? body.error : `request failed: ${response.status}`);
  }
  return body as T;
}

export function loadingState<T>(path: string): PathLoadState<T> {
  return { path, data: null, error: null, loading: true };
}

export function inferNoteRole(path: string): string {
  const name = path.split("/").at(-1) ?? "";
  if (name === "facts.md") return "facts";
  if (name === "events.md") return "events";
  if (name === "plans.md") return "plans";
  if (name === "summary.md") return "summary";
  if (name === "open_questions.md") return "open_questions";
  if (path.startsWith("/Sources/raw")) return "raw_source";
  if (path.endsWith(".md")) return "markdown_note";
  return "directory";
}

export function extractMarkdownLinks(content: string): string[] {
  const links = new Set<string>();
  const inlinePattern = /\[[^\]]+\]\(([^)]+)\)/g;
  const wikiPattern = /\[\[([^\]]+)\]\]/g;
  for (const match of content.matchAll(inlinePattern)) {
    links.add(match[1] ?? "");
  }
  for (const match of content.matchAll(wikiPattern)) {
    links.add(match[1] ?? "");
  }
  return [...links].filter(Boolean).slice(0, 20);
}

function isErrorBody(value: unknown): value is { error: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "error" in value &&
    typeof value.error === "string"
  );
}
