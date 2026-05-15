# CLI

`kinic-vfs-cli` is the shell interface for the canister-backed VFS.
This document covers wiki/database operator operations: connection, database management, node reads and writes, search, links, and archive/restore.
Skill Registry commands use the same binary under `kinic-vfs-cli skill ...`; their source of truth is [`SKILL_REGISTRY.md`](SKILL_REGISTRY.md).

The canister also exposes read-only Agent Memory API methods such as `memory_manifest`, `query_context`, and `source_evidence`.
Those are direct canister/client methods, not CLI commands in this document.
Use the CLI commands below for shell workflows against the remote VFS.

## Build

During development, examples use `cargo run` so they always execute the current checkout.
For operator use, build the binary once:

```bash
cargo build -p kinic-vfs-cli --bin kinic-vfs-cli --release
target/release/kinic-vfs-cli --help
target/release/kinic-vfs-cli --canister-id <canister-id> database current
```

GitHub Actions also produces unsigned `kinic-vfs-cli` artifacts with SHA-256 checksums. See [`RELEASE.md`](RELEASE.md).

## Connection

Use `--canister-id` to select a canister explicitly. DB-backed VFS commands require an explicit database selection from `--database-id`, `VFS_DATABASE_ID`, `.kinic/config.toml`, or user config. No production `default` database is created implicitly.
This is a breaking change for older single-DB clients that omitted `database_id`.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> --database-id <database-id> status
```

Use `--local` for the local replica host.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --local --database-id <database-id> status
```

`--database-id` takes precedence over `VFS_DATABASE_ID`.

List, search, recent, and graph commands default to the VFS root `/`.
Pass `--prefix /Wiki` or `--path /Wiki` when the human-facing wiki tree is the intended scope.

Without `--canister-id`, the CLI reads configuration from:

- `VFS_CANISTER_ID`
- `.kinic/config.toml`
- `~/.config/kinic-vfs-cli/config.toml`
- `~/.kinic-vfs-cli.toml`

Link a workspace once to avoid repeating `--database-id`:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database link <database-id>
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- database current
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- status
```

Resolution priority is CLI flag, env, `.kinic/config.toml`, user config, then host default. Use `database unlink` to remove the workspace DB link.

Create a database before reading or writing:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database create
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database list
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database grant <database-id> <principal> reader
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database link <database-id>
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- write-node --path /Wiki/file.md --input file.md
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- search-remote "budget" --prefix /Wiki --top-k 10 --json
```

`database create` prints the generated database ID.
`database list` prints databases attached to the caller principal.

For public browser reads, grant anonymous reader access explicitly:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database grant <database-id> 2vxsx-fae reader
```

## Archive and Restore

Archive exports one database as SQLite snapshot bytes and then finalizes the database into `archived` status.
Restore imports that snapshot into an `archived` or `deleted` database and returns it to `hot`.
The canister verifies the SHA-256 digest during both flows.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> \
  database archive-export <database-id> --output ./database.sqlite --json

cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> \
  database archive-restore <database-id> --input ./database.sqlite --json
```

Chunks default to 1 MiB, the canister limit. Use a smaller chunk size for local testing:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> \
  database archive-export <database-id> --output ./database.sqlite --chunk-size 65536
```

If an export fails before finalization, the CLI attempts `database archive-cancel <database-id>`.
Manual cancel is available when a database is left in `archiving`:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database archive-cancel <database-id>
```

If restore fails after it begins, the CLI attempts to cancel the restore automatically so the database returns to its previous `archived` or `deleted` state. Manual cancel is available for an interrupted restore:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id <canister-id> database restore-cancel <database-id>
```

See [`DB_LIFECYCLE.md`](DB_LIFECYCLE.md) for status, slot reuse, and restore validation details.

## Search

Full-text search uses `search-remote`.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- search-remote "budget" --prefix /Wiki --top-k 10 --json
```

Path search uses `search-path-remote`.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- search-path-remote "meeting" --prefix /Wiki --top-k 10 --json
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
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- search-path-remote "meeting" --prefix /Wiki --preview-mode content-start --json
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- search-remote "budget" --prefix /Wiki --preview-mode content-start --json
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
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- read-node-context --path /Wiki/file.md --link-limit 20 --json
```

Use graph commands for explicit link inspection.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- graph-neighborhood --center-path /Wiki/file.md --depth 1 --limit 100 --json
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- graph-links --prefix /Wiki --limit 100 --json
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- incoming-links --path /Wiki/file.md --limit 20 --json
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- outgoing-links --path /Wiki/file.md --limit 20 --json
```
