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

