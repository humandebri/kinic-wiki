export type NodeKind = "file" | "source";
export type NodeEntryKind = "file" | "source" | "directory";

export type WikiNode = {
  path: string;
  kind: NodeKind;
  content: string;
  createdAt: string;
  updatedAt: string;
  etag: string;
  metadataJson: string;
};

export type CanisterHealth = {
  cyclesBalance: bigint;
};

export type ChildNode = {
  path: string;
  name: string;
  kind: NodeEntryKind;
  updatedAt: string | null;
  etag: string | null;
  sizeBytes: string | null;
  isVirtual: boolean;
};

export type RecentNode = {
  path: string;
  kind: NodeKind;
  updatedAt: string;
  etag: string;
};

export type LinkEdge = {
  sourcePath: string;
  targetPath: string;
  rawHref: string;
  linkText: string;
  linkKind: string;
  updatedAt: string;
};

export type NodeContext = {
  node: WikiNode;
  incomingLinks: LinkEdge[];
  outgoingLinks: LinkEdge[];
};

export type SearchPreviewField = "path" | "content";

export type SearchPreview = {
  field: SearchPreviewField;
  charOffset: number;
  matchReason: string;
  excerpt: string | null;
};

export type SearchNodeHit = {
  path: string;
  kind: NodeKind;
  snippet: string | null;
  preview: SearchPreview | null;
  score: number;
  matchReasons: string[];
};
