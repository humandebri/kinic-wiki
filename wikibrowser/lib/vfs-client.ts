import { Actor, HttpAgent } from "@dfinity/agent";
import { Principal } from "@dfinity/principal";
import { sortChildNodes } from "@/lib/child-sort";
import { normalizeSearchHit, type RawSearchHit } from "@/lib/search-normalizer";
import { idlFactory } from "@/lib/vfs-idl";
import type { ChildNode, NodeEntryKind, NodeKind, RecentNode, SearchNodeHit, WikiNode } from "@/lib/types";

type Variant = Record<string, null>;

type RawNode = {
  path: string;
  kind: Variant;
  content: string;
  created_at: bigint;
  updated_at: bigint;
  etag: string;
  metadata_json: string;
};

type RawChild = {
  path: string;
  name: string;
  kind: Variant;
  updated_at: [] | [bigint];
  etag: [] | [string];
  size_bytes: [] | [bigint];
  is_virtual: boolean;
};

type RawRecent = {
  path: string;
  kind: Variant;
  updated_at: bigint;
  etag: string;
};

type VfsActor = {
  read_node: (path: string) => Promise<{ Ok: [] | [RawNode] } | { Err: string }>;
  list_children: (request: { path: string }) => Promise<{ Ok: RawChild[] } | { Err: string }>;
  recent_nodes: (request: { path: [] | [string]; limit: number }) => Promise<
    { Ok: RawRecent[] } | { Err: string }
  >;
  search_node_paths: (request: {
    query_text: string;
    prefix: [] | [string];
    top_k: number;
  }) => Promise<{ Ok: RawSearchHit[] } | { Err: string }>;
  search_nodes: (request: {
    query_text: string;
    prefix: [] | [string];
    top_k: number;
    preview_mode: [] | [Variant];
  }) => Promise<{ Ok: RawSearchHit[] } | { Err: string }>;
};

export function validateCanisterId(canisterId: string): Principal | string {
  try {
    return Principal.fromText(canisterId);
  } catch (error) {
    return error instanceof Error ? error.message : "invalid canister id";
  }
}

export async function createVfsActor(canisterId: string): Promise<VfsActor> {
  const principal = validateCanisterId(canisterId);
  if (typeof principal === "string") {
    throw new Error(principal);
  }
  const host = process.env.WIKI_IC_HOST ?? "https://icp0.io";
  const agent = await HttpAgent.create({ host });
  if (isLocalHost(host)) {
    await agent.fetchRootKey();
  }
  return Actor.createActor<VfsActor>((idl) => idlFactory(idl), {
    agent,
    canisterId: principal
  });
}

export async function readNode(canisterId: string, path: string): Promise<WikiNode | null> {
  const actor = await createVfsActor(canisterId);
  const result = await actor.read_node(path);
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  const raw = result.Ok[0];
  return raw ? normalizeNode(raw) : null;
}

export async function listChildren(canisterId: string, path: string): Promise<ChildNode[]> {
  const actor = await createVfsActor(canisterId);
  const result = await actor.list_children({ path });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  return sortChildNodes(result.Ok.map(normalizeChild));
}

export async function recentNodes(canisterId: string, limit: number): Promise<RecentNode[]> {
  const actor = await createVfsActor(canisterId);
  const result = await actor.recent_nodes({ path: [], limit });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  return result.Ok.map((node) => ({
    path: node.path,
    kind: normalizeNodeKind(node.kind),
    updatedAt: node.updated_at.toString(),
    etag: node.etag
  }));
}

export async function searchNodePaths(
  canisterId: string,
  queryText: string,
  limit: number,
  prefix: string | null
): Promise<SearchNodeHit[]> {
  const actor = await createVfsActor(canisterId);
  const result = await actor.search_node_paths({
    query_text: queryText,
    prefix: prefix ? [prefix] : [],
    top_k: limit
  });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  return result.Ok.map(normalizeSearchHit);
}

export async function searchNodes(
  canisterId: string,
  queryText: string,
  limit: number,
  prefix: string | null
): Promise<SearchNodeHit[]> {
  const actor = await createVfsActor(canisterId);
  const result = await actor.search_nodes({
    query_text: queryText,
    prefix: prefix ? [prefix] : [],
    top_k: limit,
    preview_mode: [{ Light: null }]
  });
  if ("Err" in result) {
    throw new Error(result.Err);
  }
  return result.Ok.map(normalizeSearchHit);
}

function normalizeNode(raw: RawNode): WikiNode {
  return {
    path: raw.path,
    kind: normalizeNodeKind(raw.kind),
    content: raw.content,
    createdAt: raw.created_at.toString(),
    updatedAt: raw.updated_at.toString(),
    etag: raw.etag,
    metadataJson: raw.metadata_json
  };
}

function normalizeChild(raw: RawChild): ChildNode {
  return {
    path: raw.path,
    name: raw.name,
    kind: normalizeEntryKind(raw.kind),
    updatedAt: raw.updated_at[0]?.toString() ?? null,
    etag: raw.etag[0] ?? null,
    sizeBytes: raw.size_bytes[0]?.toString() ?? null,
    isVirtual: raw.is_virtual
  };
}

function normalizeNodeKind(kind: Variant): NodeKind {
  return "Source" in kind ? "source" : "file";
}

function normalizeEntryKind(kind: Variant): NodeEntryKind {
  if ("Directory" in kind) {
    return "directory";
  }
  return "Source" in kind ? "source" : "file";
}

function isLocalHost(host: string): boolean {
  return host.includes("127.0.0.1") || host.includes("localhost");
}
