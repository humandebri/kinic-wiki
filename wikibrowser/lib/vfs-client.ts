import { Actor, HttpAgent } from "@icp-sdk/core/agent";
import { Principal } from "@icp-sdk/core/principal";
import { classifyApiError, invalidCanisterIdError } from "@/lib/api-errors";
import { sortChildNodes } from "@/lib/child-sort";
import { normalizeSearchHit, type RawSearchHit } from "@/lib/search-normalizer";
import { idlFactory } from "@/lib/vfs-idl";
import type { CanisterHealth, ChildNode, LinkEdge, NodeContext, NodeEntryKind, NodeKind, RecentNode, SearchNodeHit, WikiNode } from "@/lib/types";
import { ApiError } from "@/lib/wiki-helpers";

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

type RawCanisterHealth = {
  cycles_balance: bigint;
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

type RawLinkEdge = {
  source_path: string;
  target_path: string;
  raw_href: string;
  link_text: string;
  link_kind: string;
  updated_at: bigint;
};

type RawNodeContext = {
  node: RawNode;
  incoming_links: RawLinkEdge[];
  outgoing_links: RawLinkEdge[];
};

type VfsActor = {
  canister_health: () => Promise<RawCanisterHealth>;
  read_node: (path: string) => Promise<{ Ok: [] | [RawNode] } | { Err: string }>;
  list_children: (request: { path: string }) => Promise<{ Ok: RawChild[] } | { Err: string }>;
  recent_nodes: (request: { path: [] | [string]; limit: number }) => Promise<
    { Ok: RawRecent[] } | { Err: string }
  >;
  incoming_links: (request: { path: string; limit: number }) => Promise<{ Ok: RawLinkEdge[] } | { Err: string }>;
  outgoing_links: (request: { path: string; limit: number }) => Promise<{ Ok: RawLinkEdge[] } | { Err: string }>;
  graph_links: (request: { prefix: string; limit: number }) => Promise<{ Ok: RawLinkEdge[] } | { Err: string }>;
  graph_neighborhood: (request: { center_path: string; depth: number; limit: number }) => Promise<{ Ok: RawLinkEdge[] } | { Err: string }>;
  read_node_context: (request: { path: string; link_limit: number }) => Promise<{ Ok: [] | [RawNodeContext] } | { Err: string }>;
  search_node_paths: (request: {
    query_text: string;
    prefix: [] | [string];
    top_k: number;
    preview_mode: [] | [Variant];
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

const actorCache = new Map<string, Promise<VfsActor>>();
const healthCache = new Map<string, Promise<CanisterHealth>>();

export async function createVfsActor(canisterId: string): Promise<VfsActor> {
  const principal = validateCanisterId(canisterId);
  if (typeof principal === "string") {
    const error = invalidCanisterIdError(principal);
    throw new ApiError(error.error, 400, error.hint, error.code);
  }
  const host = process.env.NEXT_PUBLIC_WIKI_IC_HOST ?? "https://icp0.io";
  const cacheKey = `${host}\n${canisterId}`;
  const cached = actorCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const actorPromise = createActor(principal, host);
  actorCache.set(cacheKey, actorPromise);
  return actorPromise;
}

async function createActor(principal: Principal, host: string): Promise<VfsActor> {
  const agent = HttpAgent.createSync({ host });
  if (isLocalHost(host)) {
    await agent.fetchRootKey();
  }
  return Actor.createActor<VfsActor>((idl) => idlFactory(idl), {
    agent,
    canisterId: principal
  });
}

async function callVfs<T>(operation: () => Promise<T>): Promise<T> {
  try {
    return await operation();
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    const host = process.env.NEXT_PUBLIC_WIKI_IC_HOST ?? "https://icp0.io";
    const publicError = classifyApiError(error, host);
    throw new ApiError(publicError.error, 502, publicError.hint, publicError.code);
  }
}

export async function readNode(canisterId: string, path: string): Promise<WikiNode | null> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.read_node(path);
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    const raw = result.Ok[0];
    return raw ? normalizeNode(raw) : null;
  });
}

export function canisterHealth(canisterId: string): Promise<CanisterHealth> {
  const cached = healthCache.get(canisterId);
  if (cached) {
    return cached;
  }
  const request = callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    return normalizeCanisterHealth(await actor.canister_health());
  }).catch((error) => {
    healthCache.delete(canisterId);
    throw error;
  });
  healthCache.set(canisterId, request);
  return request;
}

export async function readNodeContext(canisterId: string, path: string, linkLimit: number): Promise<NodeContext | null> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.read_node_context({ path, link_limit: linkLimit });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    const raw = result.Ok[0];
    return raw ? normalizeNodeContext(raw) : null;
  });
}

export async function listChildren(canisterId: string, path: string): Promise<ChildNode[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.list_children({ path });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return sortChildNodes(result.Ok.map(normalizeChild));
  });
}

export async function recentNodes(canisterId: string, limit: number): Promise<RecentNode[]> {
  return callVfs(async () => {
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
  });
}

export async function incomingLinks(canisterId: string, path: string, limit: number): Promise<LinkEdge[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.incoming_links({ path, limit });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeLinkEdge);
  });
}

export async function outgoingLinks(canisterId: string, path: string, limit: number): Promise<LinkEdge[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.outgoing_links({ path, limit });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeLinkEdge);
  });
}

export async function graphLinks(canisterId: string, prefix: string, limit: number): Promise<LinkEdge[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.graph_links({ prefix, limit });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeLinkEdge);
  });
}

export async function graphNeighborhood(canisterId: string, centerPath: string, depth: number, limit: number): Promise<LinkEdge[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.graph_neighborhood({ center_path: centerPath, depth, limit });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeLinkEdge);
  });
}

export async function searchNodePaths(
  canisterId: string,
  queryText: string,
  limit: number,
  prefix: string | null
): Promise<SearchNodeHit[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.search_node_paths({
      query_text: queryText,
      prefix: prefix ? [prefix] : [],
      top_k: limit,
      preview_mode: [{ ContentStart: null }]
    });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeSearchHit);
  });
}

export async function searchNodes(
  canisterId: string,
  queryText: string,
  limit: number,
  prefix: string | null
): Promise<SearchNodeHit[]> {
  return callVfs(async () => {
    const actor = await createVfsActor(canisterId);
    const result = await actor.search_nodes({
      query_text: queryText,
      prefix: prefix ? [prefix] : [],
      top_k: limit,
      preview_mode: [{ ContentStart: null }]
    });
    if ("Err" in result) {
      throw new Error(result.Err);
    }
    return result.Ok.map(normalizeSearchHit);
  });
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

function normalizeCanisterHealth(raw: RawCanisterHealth): CanisterHealth {
  return {
    cyclesBalance: raw.cycles_balance
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

function normalizeLinkEdge(raw: RawLinkEdge): LinkEdge {
  return {
    sourcePath: raw.source_path,
    targetPath: raw.target_path,
    rawHref: raw.raw_href,
    linkText: raw.link_text,
    linkKind: raw.link_kind,
    updatedAt: raw.updated_at.toString()
  };
}

function normalizeNodeContext(raw: RawNodeContext): NodeContext {
  return {
    node: normalizeNode(raw.node),
    incomingLinks: raw.incoming_links.map(normalizeLinkEdge),
    outgoingLinks: raw.outgoing_links.map(normalizeLinkEdge)
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
