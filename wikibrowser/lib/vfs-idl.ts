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
  const ListChildrenRequest = idl.Record({ path: idl.Text });
  const RecentNodesRequest = idl.Record({ path: idl.Opt(idl.Text), limit: idl.Nat32 });
  const ResultNode = idl.Variant({ Ok: idl.Opt(Node), Err: idl.Text });
  const ResultChildren = idl.Variant({ Ok: idl.Vec(ChildNode), Err: idl.Text });
  const ResultRecent = idl.Variant({ Ok: idl.Vec(RecentNodeHit), Err: idl.Text });

  return idl.Service({
    read_node: idl.Func([idl.Text], [ResultNode], ["query"]),
    list_children: idl.Func([ListChildrenRequest], [ResultChildren], ["query"]),
    recent_nodes: idl.Func([RecentNodesRequest], [ResultRecent], ["query"])
  });
};
