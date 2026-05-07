// Where: extensions/conversation-capture/src/vfs-actor.js
// What: Minimal write-capable VFS actor for raw source persistence.
// Why: The wiki browser client is read-only; capture needs read_node plus write_node.
import { Actor, HttpAgent } from "@icp-sdk/core/agent";
import { IDL } from "@icp-sdk/core/candid";
import { Principal } from "@icp-sdk/core/principal";

export async function createVfsActor({ canisterId, host }) {
  const principal = Principal.fromText(canisterId);
  const agent = await HttpAgent.create({ host });
  if (isLocalHost(host)) {
    await agent.fetchRootKey();
  }
  return Actor.createActor(idlFactory, { agent, canisterId: principal });
}

function idlFactory({ IDL: idl }) {
  const NodeKind = idl.Variant({ File: idl.Null, Source: idl.Null });
  const Node = idl.Record({
    path: idl.Text,
    kind: NodeKind,
    content: idl.Text,
    created_at: idl.Int64,
    updated_at: idl.Int64,
    etag: idl.Text,
    metadata_json: idl.Text
  });
  const WriteNodeRequest = idl.Record({
    database_id: idl.Text,
    path: idl.Text,
    kind: NodeKind,
    content: idl.Text,
    metadata_json: idl.Text,
    expected_etag: idl.Opt(idl.Text)
  });
  const NodeMutationAck = idl.Record({
    path: idl.Text,
    kind: NodeKind,
    updated_at: idl.Int64,
    etag: idl.Text
  });
  const WriteNodeResult = idl.Record({ node: NodeMutationAck, created: idl.Bool });
  return idl.Service({
    read_node: idl.Func([idl.Text, idl.Text], [idl.Variant({ Ok: idl.Opt(Node), Err: idl.Text })], ["query"]),
    write_node: idl.Func([WriteNodeRequest], [idl.Variant({ Ok: WriteNodeResult, Err: idl.Text })], [])
  });
}

function isLocalHost(host) {
  return host.includes("127.0.0.1") || host.includes("localhost");
}
