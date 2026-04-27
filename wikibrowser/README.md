# Wiki Browser

Read-only browser for Kinic Wiki canisters.

## Local

```bash
pnpm install
cp .env.local.example .env.local
pnpm dev
```

Open the Phase 2 smoke node:

```text
http://localhost:3000/site/t63gs-up777-77776-aaaba-cai/Wiki/smoke-list-children/alpha.md
```

## Scope

- Browse `/Wiki` and `/Sources`
- Read Markdown notes
- Toggle preview / raw
- Inspect path, etag, update time, size, role, and outgoing links

No editing, auth, lint workflow, or full search UI in Phase 3.

## Candid Surface

`lib/vfs-idl.ts` is a small hand-written subset of `crates/vfs_canister/vfs.did`.
Keep these methods in sync when the canister interface changes:

- `read_node`
- `list_children`
- `recent_nodes`
