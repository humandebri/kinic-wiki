# llm-wiki

The best agents already think in files.  
`llm-wiki` gives them a persistent VFS backed by an IC canister and SQLite, while humans keep working in Obsidian.

The source of truth is remote `/Wiki/...` nodes.  
The local working copy is your vault's `Wiki/` folder.

## Why filesystems?

Agents already reason well about:

- reading a file
- appending to a log
- editing part of a document
- listing folders
- globbing paths
- searching memory
- moving notes when structure changes

This project keeps that file-oriented mental model, but replaces ad hoc local files with:

- a canister-backed remote store
- explicit sync
- SQLite full-text search
- `etag`-based conflict control
- an Obsidian working copy for humans

## What you get

- persistent FS-first memory on the canister
- local `Wiki/` mirror in Obsidian
- built-in search and sync
- ready-made tool schemas for OpenAI-compatible and Anthropic-compatible tool calling
- CLI commands for direct path-based operations

Current scope:

- single-tenant
- text-first
- `/Wiki/...` as the single public root

## Core operations

The public API is VFS-first.

| Operation | Meaning |
| --- | --- |
| `read_node` | Read one node by path |
| `list_nodes` | List nodes under a prefix |
| `write_node` | Replace the full content of a node |
| `append_node` | Append text to the end of a node |
| `edit_node` | Plain-text find/replace |
| `multi_edit_node` | Multiple atomic plain-text replacements |
| `mkdir_node` | Validate a directory-like path without persisting a directory row |
| `move_node` | Rename one node path |
| `delete_node` | Tombstone delete |
| `glob_nodes` | Shell-style path matching |
| `recent_nodes` | Recently updated nodes |
| `search_nodes` | Full-text search across current content |
| `export_snapshot` | Export the current snapshot |
| `fetch_updates` | Fetch changes since a known snapshot revision |

## Use with any AI SDK

This repo already ships the tool schema and dispatch layer.  
You do not need to hand-write tool definitions for every app.

The reusable pieces live in:

- client: [crates/wiki_cli/src/client.rs](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_cli/src/client.rs)
- tool layer: [crates/wiki_cli/src/agent_tools.rs](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_cli/src/agent_tools.rs)

The basic pattern is:

1. create a canister client
2. hand the SDK the generated tools
3. dispatch tool calls back into `handle_*_tool_call`

## OpenAI-compatible tool calling

```rust
use anyhow::Result;
use wiki_cli::agent_tools::{create_openai_tools, handle_openai_tool_call};
use wiki_cli::client::CanisterWikiClient;

async fn run() -> Result<()> {
    let client = CanisterWikiClient::new(
        "http://127.0.0.1:4943",
        "aaaaa-aa",
    )
    .await?;

    let tools = create_openai_tools();

    // Pass `tools` into your OpenAI-compatible SDK request.
    // When the model returns a tool call:
    let result = handle_openai_tool_call(
        &client,
        "append",
        r#"{"path":"/Wiki/memory.md","content":"remember this"}"#,
    )
    .await?;

    println!("{}", result.text);
    Ok(())
}
```

The tool names are:

- `read`
- `write`
- `append`
- `edit`
- `ls`
- `mkdir`
- `mv`
- `glob`
- `recent`
- `multi_edit`
- `rm`
- `search`

## Anthropic-compatible tool calling

```rust
use anyhow::Result;
use serde_json::json;
use wiki_cli::agent_tools::{create_anthropic_tools, handle_anthropic_tool_call};
use wiki_cli::client::CanisterWikiClient;

async fn run() -> Result<()> {
    let client = CanisterWikiClient::new(
        "http://127.0.0.1:4943",
        "aaaaa-aa",
    )
    .await?;

    let tools = create_anthropic_tools();

    // Pass `tools` into your Anthropic-compatible SDK request.
    // When the model returns a tool_use block:
    let result = handle_anthropic_tool_call(
        &client,
        "ls",
        json!({
            "prefix": "/Wiki",
            "recursive": false,
            "include_deleted": false
        }),
    )
    .await?;

    println!("{}", result.text);
    Ok(())
}
```

## Direct tool access

If you do not want to integrate through a chat SDK yet, you can still use the same layer directly.

```rust
use anyhow::Result;
use serde_json::json;
use wiki_cli::agent_tools::handle_anthropic_tool_call;
use wiki_cli::client::CanisterWikiClient;

async fn run() -> Result<()> {
    let client = CanisterWikiClient::new(
        "http://127.0.0.1:4943",
        "aaaaa-aa",
    )
    .await?;

    let result = handle_anthropic_tool_call(
        &client,
        "search",
        json!({
            "query_text": "architecture",
            "prefix": "/Wiki",
            "top_k": 5
        }),
    )
    .await?;

    println!("{}", result.text);
    Ok(())
}
```

## Use with the CLI

The CLI lives in [crates/wiki_cli](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_cli).

Main commands:

- `read-node`
- `list-nodes`
- `write-node`
- `append-node`
- `edit-node`
- `multi-edit-node`
- `mkdir-node`
- `move-node`
- `glob-nodes`
- `recent-nodes`
- `delete-node`
- `search-remote`
- `status`
- `lint-local`
- `pull`
- `push`

Examples:

```bash
wiki-cli read-node --path /Wiki/notes.md
wiki-cli glob-nodes '**/*.md' --path /Wiki --type file
wiki-cli recent-nodes --limit 20 --path /Wiki
wiki-cli append-node --path /Wiki/log.md --input ./entry.md
wiki-cli move-node --from-path /Wiki/draft.md --to-path /Wiki/archive/draft.md --expected-etag etag-1
```

## Use with Obsidian

The plugin lives in [plugins/kinic-wiki](/Users/0xhude/Desktop/work/llm-wiki/plugins/kinic-wiki).

It is responsible for:

- pulling remote nodes into `Wiki/`
- pushing local changes back to the canister
- deleting mirrored files remotely
- writing conflict notes when `etag` mismatches occur

Mirror mapping:

- remote `/Wiki/foo.md` -> local `Wiki/foo.md`
- remote `/Wiki/nested/bar.md` -> local `Wiki/nested/bar.md`
- conflict file -> `Wiki/conflicts/<basename>.conflict.md`

Managed mirror frontmatter:

- `path`
- `kind`
- `etag`
- `updated_at`
- `mirror: true`

The plugin is still sync-first.  
It already understands the new VFS API shape, but it does not yet expose separate UI commands for `append`, `glob`, `recent`, or `multi_edit`.

## Data model

The source of truth is a `Node`.

Main fields:

- `path`
- `kind`
- `content`
- `created_at`
- `updated_at`
- `etag`
- `deleted_at`
- `metadata_json`

Persisted node kinds:

- `file`
- `source`

List-only entry kinds:

- `directory`
- `file`
- `source`

Directories are virtual.
They appear in `list_nodes` and `glob_nodes`, but they are not persisted as rows.

Main tables:

- `fs_nodes`
- `fs_nodes_fts`
- `fs_change_log`

## Conflict model

Concurrency is controlled with `etag`, not revision IDs.

- create: `expected_etag = None`
- update: current `etag` must match
- delete: current `etag` must match
- stale writes fail instead of silently merging

Deletes are tombstones.
Moves are path renames, not copy-plus-delete at the API boundary.

## Search

Search is built into the same SQLite store.

- backend: SQLite FTS
- target: current non-deleted content
- prefix scoping supported

There is no separate canister-only search implementation.

## Sync

Sync uses:

- `export_snapshot` for full export
- `fetch_updates` for delta sync

If the client does not know a valid snapshot revision, `fetch_updates` returns a full refresh instead of erroring.

For rename operations, sync observes:

- new path in `changed_nodes`
- old path in `removed_paths`

## Current architecture

- canister: [crates/wiki_canister](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_canister)
- runtime: [crates/wiki_runtime](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_runtime)
- store: [crates/wiki_store](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_store)
- public types: [crates/wiki_types](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_types)
- CLI: [crates/wiki_cli](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_cli)
- Obsidian plugin: [plugins/kinic-wiki](/Users/0xhude/Desktop/work/llm-wiki/plugins/kinic-wiki)

Important implementation notes:

- search, sync, and writes share the same SQLite database
- canister init / post-upgrade runs FS migrations only
- the public contract is defined in [crates/wiki_canister/wiki.did](/Users/0xhude/Desktop/work/llm-wiki/crates/wiki_canister/wiki.did)

## Build

Canister build script:
[scripts/build-wiki-canister.sh](/Users/0xhude/Desktop/work/llm-wiki/scripts/build-wiki-canister.sh)

Build flow:

1. `cargo build --target wasm32-wasip1 -p wiki-canister`
2. `wasi2ic`
3. `ic-wasm` embeds `candid:service`

Project build config:
[icp.yaml](/Users/0xhude/Desktop/work/llm-wiki/icp.yaml)

## Development checks

Rust:

```bash
cargo test
cargo build --target wasm32-wasip1 -p wiki-canister
ICP_WASM_OUTPUT_PATH=/tmp/wiki_canister_test.wasm bash scripts/build-wiki-canister.sh
```

Plugin:

```bash
cd plugins/kinic-wiki
npm run check
```

## Not in scope

These are intentionally not implemented right now:

- `tag`
- binary node support
- multi-tenant wire shape
- directory tree move
- persisted directory rows

This project is not trying to be a full POSIX filesystem.  
It is trying to be a practical, file-oriented memory layer for agents and humans sharing the same knowledge space.
