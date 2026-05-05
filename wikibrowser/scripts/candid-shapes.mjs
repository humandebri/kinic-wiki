export const expectedTypes = {
  CanisterHealth: { kind: "record", fields: { cycles_balance: "nat" } },
  CanonicalRole: {
    kind: "record",
    fields: { name: "text", path_pattern: "text", purpose: "text" }
  },
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
  MemoryCapability: { kind: "record", fields: { name: "text", description: "text" } },
  MemoryManifest: {
    kind: "record",
    fields: {
      api_version: "text",
      budget_unit: "text",
      capabilities: "vec MemoryCapability",
      max_depth: "nat32",
      max_query_limit: "nat32",
      recommended_entrypoint: "text",
      write_policy: "text",
      canonical_roles: "vec CanonicalRole",
      purpose: "text",
      roots: "vec MemoryRoot"
    }
  },
  MemoryRoot: { kind: "record", fields: { kind: "text", path: "text" } },
  QueryContext: {
    kind: "record",
    fields: {
      truncated: "bool",
      task: "text",
      evidence: "vec SourceEvidence",
      nodes: "vec NodeContext",
      graph_links: "vec LinkEdge",
      search_hits: "vec SearchNodeHit",
      namespace: "text"
    }
  },
  QueryContextRequest: {
    kind: "record",
    fields: {
      task: "text",
      include_evidence: "bool",
      entities: "vec text",
      budget_tokens: "nat32",
      depth: "nat32",
      namespace: "opt text"
    }
  },
  RecentNodeHit: {
    kind: "record",
    fields: { updated_at: "int64", etag: "text", kind: "NodeKind", path: "text" }
  },
  RecentNodesRequest: { kind: "record", fields: { path: "opt text", limit: "nat32" } },
  GraphLinksRequest: { kind: "record", fields: { limit: "nat32", prefix: "text" } },
  GraphNeighborhoodRequest: { kind: "record", fields: { center_path: "text", limit: "nat32", depth: "nat32" } },
  IncomingLinksRequest: { kind: "record", fields: { path: "text", limit: "nat32" } },
  NodeContextRequest: { kind: "record", fields: { link_limit: "nat32", path: "text" } },
  OutgoingLinksRequest: { kind: "record", fields: { path: "text", limit: "nat32" } },
  LinkEdge: {
    kind: "record",
    fields: {
      updated_at: "int64",
      link_kind: "text",
      link_text: "text",
      source_path: "text",
      raw_href: "text",
      target_path: "text"
    }
  },
  NodeContext: {
    kind: "record",
    fields: { incoming_links: "vec LinkEdge", node: "Node", outgoing_links: "vec LinkEdge" }
  },
  ResultChildren: { kind: "variant", cases: { Ok: "vec ChildNode", Err: "text" } },
  ResultLinks: { kind: "variant", cases: { Ok: "vec LinkEdge", Err: "text" } },
  ResultNode: { kind: "variant", cases: { Ok: "opt Node", Err: "text" } },
  ResultNodeContext: { kind: "variant", cases: { Ok: "opt NodeContext", Err: "text" } },
  ResultQueryContext: { kind: "variant", cases: { Ok: "QueryContext", Err: "text" } },
  ResultRecent: { kind: "variant", cases: { Ok: "vec RecentNodeHit", Err: "text" } },
  ResultSearch: { kind: "variant", cases: { Ok: "vec SearchNodeHit", Err: "text" } },
  ResultPathPolicyEntries: { kind: "variant", cases: { Ok: "vec PathPolicyEntry", Err: "text" } },
  ResultSourceEvidence: { kind: "variant", cases: { Ok: "SourceEvidence", Err: "text" } },
  ResultUnit: { kind: "variant", cases: { Ok: "null", Err: "text" } },
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
    fields: {
      top_k: "nat32",
      preview_mode: "opt SearchPreviewMode",
      prefix: "opt text",
      query_text: "text"
    }
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
  SearchPreviewMode: { kind: "variant", cases: { Light: "null", ContentStart: "null", None: "null" } },
  PathPolicyEntry: { kind: "record", fields: { principal: "text", roles: "vec text" } },
  PathPolicy: { kind: "record", fields: { mode: "text", path: "text", roles: "vec text" } },
  SourceEvidence: {
    kind: "record",
    fields: { node_path: "text", refs: "vec SourceEvidenceRef" }
  },
  SourceEvidenceRef: {
    kind: "record",
    fields: {
      link_text: "text",
      via_path: "text",
      source_path: "text",
      raw_href: "text"
    }
  },
  SourceEvidenceRequest: { kind: "record", fields: { node_path: "text" } }
};

export const didTypeAliases = {
  ResultChildren: "Result_9",
  ResultLinks: "Result_8",
  ResultNode: "Result_15",
  ResultNodeContext: "Result_16",
  ResultPathPolicyEntries: "Result_13",
  ResultQueryContext: "Result_14",
  ResultRecent: "Result_17",
  ResultSearch: "Result_18",
  ResultSourceEvidence: "Result_19",
  ResultUnit: "Result_7"
};

export const expectedMethods = {
  canister_health: { input: [], output: "CanisterHealth", mode: "query" },
  graph_links: { input: ["GraphLinksRequest"], output: "ResultLinks", mode: "query" },
  graph_neighborhood: { input: ["GraphNeighborhoodRequest"], output: "ResultLinks", mode: "query" },
  grant_path_policy_role: { input: ["text", "text", "text"], output: "ResultUnit", mode: "update" },
  incoming_links: { input: ["IncomingLinksRequest"], output: "ResultLinks", mode: "query" },
  list_children: { input: ["ListChildrenRequest"], output: "ResultChildren", mode: "query" },
  memory_manifest: { input: [], output: "MemoryManifest", mode: "query" },
  my_path_policy_roles: { input: ["text"], output: "PrincipalRoles", mode: "query" },
  outgoing_links: { input: ["OutgoingLinksRequest"], output: "ResultLinks", mode: "query" },
  query_context: { input: ["QueryContextRequest"], output: "ResultQueryContext", mode: "query" },
  read_node: { input: ["text"], output: "ResultNode", mode: "query" },
  read_node_context: { input: ["NodeContextRequest"], output: "ResultNodeContext", mode: "query" },
  recent_nodes: { input: ["RecentNodesRequest"], output: "ResultRecent", mode: "query" },
  revoke_path_policy_role: { input: ["text", "text", "text"], output: "ResultUnit", mode: "update" },
  search_node_paths: { input: ["SearchNodePathsRequest"], output: "ResultSearch", mode: "query" },
  search_nodes: { input: ["SearchNodesRequest"], output: "ResultSearch", mode: "query" },
  path_policy_entries: { input: ["text"], output: "ResultPathPolicyEntries", mode: "query" },
  path_policy: { input: ["text"], output: "PathPolicy", mode: "query" },
  source_evidence: { input: ["SourceEvidenceRequest"], output: "ResultSourceEvidence", mode: "query" }
};
