# CLI

`vfs-cli` is the shell interface for the canister-backed VFS.

The canister also exposes read-only Agent Memory API methods such as `memory_manifest`, `query_context`, and `source_evidence`.
Those are direct canister/client methods, not CLI commands in this document.
Use the CLI commands below for shell workflows and local mirror operations.

## Connection

Use `--canister-id` to select a canister explicitly. DB-backed VFS commands require `--database-id` or `VFS_DATABASE_ID`; no production `default` database is created implicitly.
This is a breaking change for older single-DB clients that omitted `database_id`.

```bash
cargo run -p vfs-cli -- --canister-id <canister-id> --database-id <database-id> status
```

Use `--local` for the local replica host.

```bash
cargo run -p vfs-cli -- --local --database-id <database-id> status
```

`--database-id` takes precedence over `VFS_DATABASE_ID`.

List, search, recent, and graph commands default to the VFS root `/`.
Pass `--prefix /Wiki` or `--path /Wiki` when the human-facing wiki tree is the intended scope.

Without `--canister-id`, the CLI reads configuration from:

- `VFS_CANISTER_ID`
- `~/.config/vfs-cli/config.toml`
- `~/.vfs-cli.toml`

Create a database before reading or writing:

```bash
cargo run -p vfs-cli -- --canister-id <canister-id> database create
cargo run -p vfs-cli -- --canister-id <canister-id> database grant <database-id> <principal> reader
cargo run -p vfs-cli -- --canister-id <canister-id> --database-id <database-id> write-node --path /Wiki/file.md --input file.md
cargo run -p vfs-cli -- --canister-id <canister-id> --database-id <database-id> search-remote "budget" --prefix /Wiki --top-k 10 --json
```

`database create` prints the generated database ID.

For public browser reads, grant anonymous reader access explicitly:

```bash
cargo run -p vfs-cli -- --canister-id <canister-id> database grant <database-id> 2vxsx-fae reader
```

Archive and restore are low-level canister APIs for snapshot bytes. The CLI does not yet persist archive bytes for you. See [`DB_LIFECYCLE.md`](DB_LIFECYCLE.md) for status, slot reuse, and restore validation details.

If `pull` or `push` reports a snapshot revision resync condition, run `pull --resync` before retrying the workflow.

## Search

Full-text search uses `search-remote`.

```bash
cargo run -p vfs-cli -- search-remote "budget" --prefix /Wiki --top-k 10 --json
```

Path search uses `search-path-remote`.

```bash
cargo run -p vfs-cli -- search-path-remote "meeting" --prefix /Wiki --top-k 10 --json
```

`--preview-mode` is optional. If omitted, canister defaults are preserved:

- `search-remote`: light match preview
- `search-path-remote`: no preview

Available preview modes:

- `none`: no `SearchNodeHit.preview`
- `light`: match-oriented preview
- `content-start`: body-start preview in `SearchNodeHit.preview.excerpt`

Use `content-start` when the caller needs the first 200 normalized body characters without an extra `read-node` call.

```bash
cargo run -p vfs-cli -- search-path-remote "meeting" --prefix /Wiki --preview-mode content-start --json
cargo run -p vfs-cli -- search-remote "budget" --prefix /Wiki --preview-mode content-start --json
```

## Node Operations

Common read and write commands:

- `read-node --path /Wiki/file.md`
- `read-node-context --path /Wiki/file.md --link-limit 20 --json`
- `list-children --path /Wiki --json`
- `write-node --path /Wiki/file.md --input file.md`
- `append-node --path /Wiki/file.md --input append.md`
- `edit-node --path /Wiki/file.md --old-text before --new-text after`
- `delete-node --path /Wiki/file.md`
- `move-node --from-path /Wiki/a.md --to-path /Wiki/b.md`
- `glob-nodes "**/*.md" --path /Wiki --json`
- `recent-nodes 20 --path /Wiki --json`

## Link Graph

Use `read-node-context` when the caller needs a node plus incoming and outgoing links in one response.

```bash
cargo run -p vfs-cli -- read-node-context --path /Wiki/file.md --link-limit 20 --json
```

Use graph commands for explicit link inspection.

```bash
cargo run -p vfs-cli -- graph-neighborhood --center-path /Wiki/file.md --depth 1 --limit 100 --json
cargo run -p vfs-cli -- graph-links --prefix /Wiki --limit 100 --json
cargo run -p vfs-cli -- incoming-links --path /Wiki/file.md --limit 20 --json
cargo run -p vfs-cli -- outgoing-links --path /Wiki/file.md --limit 20 --json
```

## Mirror Operations

Use `pull` and `push` for local mirror sync.

```bash
cargo run -p vfs-cli -- pull --vault-path ./vault
cargo run -p vfs-cli -- push --vault-path ./vault
```
