export const expectedTypes = {
  CanisterHealth: { kind: "record", fields: { cycles_balance: "nat" } },
  DatabaseRole: { kind: "variant", cases: { Reader: "null", Writer: "null", Owner: "null" } },
  DatabaseStatus: { kind: "variant", cases: { Hot: "null", Restoring: "null", Archiving: "null", Archived: "null", Deleted: "null" } },
  DatabaseSummary: {
    kind: "record",
    fields: {
      status: "DatabaseStatus",
      role: "DatabaseRole",
      logical_size_bytes: "nat64",
      database_id: "text",
      archived_at_ms: "opt int64",
      deleted_at_ms: "opt int64"
    }
  },
  DatabaseMember: {
    kind: "record",
    fields: {
      principal: "text",
      role: "DatabaseRole",
      created_at_ms: "int64",
      database_id: "text"
    }
  },
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
      has_children: "bool",
      is_virtual: "bool"
    }
  },
  ListChildrenRequest: { kind: "record", fields: { path: "text", database_id: "text" } },
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
      database_id: "text",
      depth: "nat32",
      namespace: "opt text"
    }
  },
  RecentNodeHit: {
    kind: "record",
    fields: { updated_at: "int64", etag: "text", kind: "NodeKind", path: "text" }
  },
  RecentNodesRequest: { kind: "record", fields: { path: "opt text", limit: "nat32", database_id: "text" } },
  GraphLinksRequest: { kind: "record", fields: { limit: "nat32", database_id: "text", prefix: "text" } },
  GraphNeighborhoodRequest: { kind: "record", fields: { center_path: "text", limit: "nat32", database_id: "text", depth: "nat32" } },
  IncomingLinksRequest: { kind: "record", fields: { path: "text", limit: "nat32", database_id: "text" } },
  NodeContextRequest: { kind: "record", fields: { link_limit: "nat32", path: "text", database_id: "text" } },
  OutgoingLinksRequest: { kind: "record", fields: { path: "text", limit: "nat32", database_id: "text" } },
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
  ResultCreateDatabase: { kind: "variant", cases: { Ok: "text", Err: "text" } },
  ResultDatabases: { kind: "variant", cases: { Ok: "vec DatabaseSummary", Err: "text" } },
  ResultMembers: { kind: "variant", cases: { Ok: "vec DatabaseMember", Err: "text" } },
  ResultUnit: { kind: "variant", cases: { Ok: "null", Err: "text" } },
  ResultLinks: { kind: "variant", cases: { Ok: "vec LinkEdge", Err: "text" } },
  ResultNode: { kind: "variant", cases: { Ok: "opt Node", Err: "text" } },
  ResultNodeContext: { kind: "variant", cases: { Ok: "opt NodeContext", Err: "text" } },
  ResultQueryContext: { kind: "variant", cases: { Ok: "QueryContext", Err: "text" } },
  ResultRecent: { kind: "variant", cases: { Ok: "vec RecentNodeHit", Err: "text" } },
  ResultSearch: { kind: "variant", cases: { Ok: "vec SearchNodeHit", Err: "text" } },
  ResultSourceEvidence: { kind: "variant", cases: { Ok: "SourceEvidence", Err: "text" } },
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
      database_id: "text",
      preview_mode: "opt SearchPreviewMode",
      prefix: "opt text",
      query_text: "text"
    }
  },
  SearchNodesRequest: {
    kind: "record",
    fields: {
      top_k: "nat32",
      database_id: "text",
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
  SourceEvidenceRequest: { kind: "record", fields: { node_path: "text", database_id: "text" } }
};

export const didTypeAliases = {
  ResultChildren: "Result_10",
  ResultCreateDatabase: "Result_3",
  ResultDatabases: "Result_12",
  ResultMembers: "Result_11",
  ResultUnit: "Result_2",
  ResultLinks: "Result_9",
  ResultNode: "Result_18",
  ResultNodeContext: "Result_19",
  ResultQueryContext: "Result_16",
  ResultRecent: "Result_20",
  ResultSearch: "Result_21",
  ResultSourceEvidence: "Result_22"
};

export const expectedMethods = {
  canister_health: { input: [], output: "CanisterHealth", mode: "query" },
  create_database: { input: [], output: "ResultCreateDatabase", mode: "update" },
  grant_database_access: { input: ["text", "text", "DatabaseRole"], output: "ResultUnit", mode: "update" },
  graph_links: { input: ["GraphLinksRequest"], output: "ResultLinks", mode: "query" },
  graph_neighborhood: { input: ["GraphNeighborhoodRequest"], output: "ResultLinks", mode: "query" },
  incoming_links: { input: ["IncomingLinksRequest"], output: "ResultLinks", mode: "query" },
  list_children: { input: ["ListChildrenRequest"], output: "ResultChildren", mode: "query" },
  list_databases: { input: [], output: "ResultDatabases", mode: "query" },
  list_database_members: { input: ["text"], output: "ResultMembers", mode: "query" },
  memory_manifest: { input: [], output: "MemoryManifest", mode: "query" },
  outgoing_links: { input: ["OutgoingLinksRequest"], output: "ResultLinks", mode: "query" },
  query_context: { input: ["QueryContextRequest"], output: "ResultQueryContext", mode: "query" },
  read_node: { input: ["text", "text"], output: "ResultNode", mode: "query" },
  read_node_context: { input: ["NodeContextRequest"], output: "ResultNodeContext", mode: "query" },
  recent_nodes: { input: ["RecentNodesRequest"], output: "ResultRecent", mode: "query" },
  revoke_database_access: { input: ["text", "text"], output: "ResultUnit", mode: "update" },
  search_node_paths: { input: ["SearchNodePathsRequest"], output: "ResultSearch", mode: "query" },
  search_nodes: { input: ["SearchNodesRequest"], output: "ResultSearch", mode: "query" },
  source_evidence: { input: ["SourceEvidenceRequest"], output: "ResultSourceEvidence", mode: "query" }
};
