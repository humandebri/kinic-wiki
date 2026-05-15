import type { Identity } from "@icp-sdk/core/agent";
import type { WikiNode } from "@/lib/types";
import { readNode, searchNodes } from "@/lib/vfs-client";

export type OpsAnswerContext = {
  path: string;
  title: string;
  excerpt: string;
};

const MAX_CONTEXT_ITEMS = 6;
const MAX_CONTEXT_CHARS = 18_000;
const MAX_EXCERPT_CHARS = 4_000;

export async function collectOpsAnswerContext(input: {
  canisterId: string;
  databaseId: string;
  question: string;
  selectedPath: string;
  currentNode: WikiNode | null;
  readIdentity: Identity | null;
}): Promise<OpsAnswerContext[]> {
  const nodes = new Map<string, WikiNode>();
  if (input.currentNode && isContextPath(input.currentNode.path)) {
    nodes.set(input.currentNode.path, input.currentNode);
  }
  const hits = await searchNodes(input.canisterId, input.databaseId, input.question, MAX_CONTEXT_ITEMS, "/Wiki", input.readIdentity ?? undefined);
  for (const hit of hits) {
    if (nodes.size >= MAX_CONTEXT_ITEMS) break;
    if (!isContextPath(hit.path) || nodes.has(hit.path)) continue;
    const node = await readNode(input.canisterId, input.databaseId, hit.path, input.readIdentity ?? undefined);
    if (node && node.kind === "file") nodes.set(node.path, node);
  }
  return trimContext([...nodes.values()].map(contextFromNode));
}

function contextFromNode(node: WikiNode): OpsAnswerContext {
  return {
    path: node.path,
    title: titleForNode(node),
    excerpt: node.content.slice(0, MAX_EXCERPT_CHARS)
  };
}

function trimContext(items: OpsAnswerContext[]): OpsAnswerContext[] {
  const trimmed: OpsAnswerContext[] = [];
  let total = 0;
  for (const item of items) {
    if (total >= MAX_CONTEXT_CHARS) break;
    const remaining = MAX_CONTEXT_CHARS - total;
    const excerpt = item.excerpt.slice(0, remaining);
    total += excerpt.length;
    trimmed.push({ ...item, excerpt });
  }
  return trimmed;
}

function titleForNode(node: WikiNode): string {
  const heading = node.content.split("\n").find((line) => line.startsWith("# "));
  return heading ? heading.slice(2).trim() : node.path.split("/").at(-1) ?? node.path;
}

function isContextPath(path: string): boolean {
  return path === "/Wiki" || path.startsWith("/Wiki/") || path === "/Sources" || path.startsWith("/Sources/");
}
