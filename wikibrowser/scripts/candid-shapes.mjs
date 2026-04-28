export const expectedTypes = {
  ChildNode: {
    kind: "record",
    fields: {
      updated_at: "opt int64",
      etag: "opt text",
      kind: "NodeEntryKind",
      name: "text",
      size_bytes: "opt nat64",
      path: "text",
      is_virtual: "bool"
    }
  },
  ListChildrenRequest: { kind: "record", fields: { path: "text" } },
  Node: {
    kind: "record",
    fields: {
      updated_at: "int64",
      content: "text",
      etag: "text",
      kind: "NodeKind",
      path: "text",
      created_at: "int64",
      metadata_json: "text"
    }
  },
  NodeEntryKind: { kind: "variant", cases: { File: "null", Source: "null", Directory: "null" } },
  NodeKind: { kind: "variant", cases: { File: "null", Source: "null" } },
  RecentNodeHit: {
    kind: "record",
    fields: { updated_at: "int64", etag: "text", kind: "NodeKind", path: "text" }
  },
  RecentNodesRequest: { kind: "record", fields: { path: "opt text", limit: "nat32" } },
  ResultChildren: { kind: "variant", cases: { Ok: "vec ChildNode", Err: "text" } },
  ResultNode: { kind: "variant", cases: { Ok: "opt Node", Err: "text" } },
  ResultRecent: { kind: "variant", cases: { Ok: "vec RecentNodeHit", Err: "text" } },
  ResultSearch: { kind: "variant", cases: { Ok: "vec SearchNodeHit", Err: "text" } },
  SearchNodeHit: {
    kind: "record",
    fields: {
      preview: "opt SearchPreview",
      kind: "NodeKind",
      path: "text",
      match_reasons: "vec text",
      snippet: "opt text",
      score: "float32"
    }
  },
  SearchNodePathsRequest: {
    kind: "record",
    fields: { top_k: "nat32", prefix: "opt text", query_text: "text" }
  },
  SearchNodesRequest: {
    kind: "record",
    fields: {
      top_k: "nat32",
      preview_mode: "opt SearchPreviewMode",
      prefix: "opt text",
      query_text: "text"
    }
  },
  SearchPreview: {
    kind: "record",
    fields: {
      field: "SearchPreviewField",
      char_offset: "nat32",
      match_reason: "text",
      excerpt: "opt text"
    }
  },
  SearchPreviewField: { kind: "variant", cases: { Path: "null", Content: "null" } },
  SearchPreviewMode: { kind: "variant", cases: { Light: "null", None: "null" } }
};

export const didTypeAliases = {
  ResultChildren: "Result_6",
  ResultNode: "Result_10",
  ResultRecent: "Result_11",
  ResultSearch: "Result_12"
};

export const expectedMethods = {
  list_children: { input: ["ListChildrenRequest"], output: "ResultChildren", mode: "query" },
  read_node: { input: ["text"], output: "ResultNode", mode: "query" },
  recent_nodes: { input: ["RecentNodesRequest"], output: "ResultRecent", mode: "query" },
  search_node_paths: { input: ["SearchNodePathsRequest"], output: "ResultSearch", mode: "query" },
  search_nodes: { input: ["SearchNodesRequest"], output: "ResultSearch", mode: "query" }
};
