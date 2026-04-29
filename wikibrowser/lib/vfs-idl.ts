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
  const LinkEdge = idl.Record({
    source_path: idl.Text,
    target_path: idl.Text,
    raw_href: idl.Text,
    link_text: idl.Text,
    link_kind: idl.Text,
    updated_at: idl.Int64
  });
  const NodeContext = idl.Record({
    incoming_links: idl.Vec(LinkEdge),
    node: Node,
    outgoing_links: idl.Vec(LinkEdge)
  });
  const SearchPreviewField = idl.Variant({ Path: idl.Null, Content: idl.Null });
  const SearchPreviewMode = idl.Variant({ Light: idl.Null, ContentStart: idl.Null, None: idl.Null });
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
  const IncomingLinksRequest = idl.Record({ path: idl.Text, limit: idl.Nat32 });
  const OutgoingLinksRequest = idl.Record({ path: idl.Text, limit: idl.Nat32 });
  const GraphLinksRequest = idl.Record({ prefix: idl.Text, limit: idl.Nat32 });
  const GraphNeighborhoodRequest = idl.Record({ center_path: idl.Text, depth: idl.Nat32, limit: idl.Nat32 });
  const NodeContextRequest = idl.Record({ path: idl.Text, link_limit: idl.Nat32 });
  const SearchNodePathsRequest = idl.Record({
    query_text: idl.Text,
    prefix: idl.Opt(idl.Text),
    top_k: idl.Nat32,
    preview_mode: idl.Opt(SearchPreviewMode)
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
  const ResultLinks = idl.Variant({ Ok: idl.Vec(LinkEdge), Err: idl.Text });
  const ResultNodeContext = idl.Variant({ Ok: idl.Opt(NodeContext), Err: idl.Text });
  const ResultSearch = idl.Variant({ Ok: idl.Vec(SearchNodeHit), Err: idl.Text });

  return idl.Service({
    graph_links: idl.Func([GraphLinksRequest], [ResultLinks], ["query"]),
    graph_neighborhood: idl.Func([GraphNeighborhoodRequest], [ResultLinks], ["query"]),
    incoming_links: idl.Func([IncomingLinksRequest], [ResultLinks], ["query"]),
    read_node: idl.Func([idl.Text], [ResultNode], ["query"]),
    read_node_context: idl.Func([NodeContextRequest], [ResultNodeContext], ["query"]),
    list_children: idl.Func([ListChildrenRequest], [ResultChildren], ["query"]),
    outgoing_links: idl.Func([OutgoingLinksRequest], [ResultLinks], ["query"]),
    recent_nodes: idl.Func([RecentNodesRequest], [ResultRecent], ["query"]),
    search_node_paths: idl.Func([SearchNodePathsRequest], [ResultSearch], ["query"]),
    search_nodes: idl.Func([SearchNodesRequest], [ResultSearch], ["query"])
  });
};
