import { IDL } from "@dfinity/candid";
import { Actor } from "@dfinity/agent";

type ActorInterfaceFactory = Parameters<typeof Actor.createActor>[0];

export const idlFactory: ActorInterfaceFactory = ({ IDL: idl }) => {
  const NodeKind = idl.Variant({ File: idl.Null, Source: idl.Null });
  const NodeEntryKind = idl.Variant({
    File: idl.Null,
    Source: idl.Null,
    Directory: idl.Null
  });
  const Node = idl.Record({
    path: idl.Text,
    kind: NodeKind,
    content: idl.Text,
    created_at: idl.Int64,
    updated_at: idl.Int64,
    etag: idl.Text,
    metadata_json: idl.Text
  });
  const ChildNode = idl.Record({
    path: idl.Text,
    name: idl.Text,
    kind: NodeEntryKind,
    updated_at: idl.Opt(idl.Int64),
    etag: idl.Opt(idl.Text),
    size_bytes: idl.Opt(idl.Nat64),
    is_virtual: idl.Bool
  });
  const RecentNodeHit = idl.Record({
    path: idl.Text,
    kind: NodeKind,
    updated_at: idl.Int64,
    etag: idl.Text
  });
  const SearchPreviewField = idl.Variant({ Path: idl.Null, Content: idl.Null });
  const SearchPreviewMode = idl.Variant({ Light: idl.Null, None: idl.Null });
  const SearchPreview = idl.Record({
    field: SearchPreviewField,
    char_offset: idl.Nat32,
    match_reason: idl.Text,
    excerpt: idl.Opt(idl.Text)
  });
  const SearchNodeHit = idl.Record({
    path: idl.Text,
    kind: NodeKind,
    snippet: idl.Opt(idl.Text),
    preview: idl.Opt(SearchPreview),
    score: idl.Float32,
    match_reasons: idl.Vec(idl.Text)
  });
  const ListChildrenRequest = idl.Record({ path: idl.Text });
  const RecentNodesRequest = idl.Record({ path: idl.Opt(idl.Text), limit: idl.Nat32 });
  const SearchNodePathsRequest = idl.Record({
    query_text: idl.Text,
    prefix: idl.Opt(idl.Text),
    top_k: idl.Nat32
  });
  const SearchNodesRequest = idl.Record({
    query_text: idl.Text,
    prefix: idl.Opt(idl.Text),
    top_k: idl.Nat32,
    preview_mode: idl.Opt(SearchPreviewMode)
  });
  const ResultNode = idl.Variant({ Ok: idl.Opt(Node), Err: idl.Text });
  const ResultChildren = idl.Variant({ Ok: idl.Vec(ChildNode), Err: idl.Text });
  const ResultRecent = idl.Variant({ Ok: idl.Vec(RecentNodeHit), Err: idl.Text });
  const ResultSearch = idl.Variant({ Ok: idl.Vec(SearchNodeHit), Err: idl.Text });

  return idl.Service({
    read_node: idl.Func([idl.Text], [ResultNode], ["query"]),
    list_children: idl.Func([ListChildrenRequest], [ResultChildren], ["query"]),
    recent_nodes: idl.Func([RecentNodesRequest], [ResultRecent], ["query"]),
    search_node_paths: idl.Func([SearchNodePathsRequest], [ResultSearch], ["query"]),
    search_nodes: idl.Func([SearchNodesRequest], [ResultSearch], ["query"])
  });
};
