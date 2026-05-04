# CLI

`vfs-cli` is the shell interface for the canister-backed VFS.

The canister also exposes read-only Agent Memory API methods such as `memory_manifest`, `query_context`, and `source_evidence`.
Those are direct canister/client methods, not CLI commands in this document.
Use the CLI commands below for shell workflows and local mirror operations.

## Connection

Use `--canister-id` to select a canister explicitly.

```bash
cargo run -p vfs-cli --bin vfs-cli -- --canister-id <canister-id> status
```

Use `--local` for the local replica host.

```bash
cargo run -p vfs-cli --bin vfs-cli -- --local status
```

Without `--canister-id`, the CLI reads configuration from:

- `VFS_CANISTER_ID`
- `~/.config/vfs-cli/config.toml`
- `~/.vfs-cli.toml`

## Search

Full-text search uses `search-remote`.

```bash
cargo run -p vfs-cli --bin vfs-cli -- search-remote "budget" --prefix /Wiki --top-k 10 --json
```

Path search uses `search-path-remote`.

```bash
cargo run -p vfs-cli --bin vfs-cli -- search-path-remote "meeting" --prefix /Wiki --top-k 10 --json
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
cargo run -p vfs-cli --bin vfs-cli -- search-path-remote "meeting" --prefix /Wiki --preview-mode content-start --json
cargo run -p vfs-cli --bin vfs-cli -- search-remote "budget" --prefix /Wiki --preview-mode content-start --json
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
cargo run -p vfs-cli --bin vfs-cli -- read-node-context --path /Wiki/file.md --link-limit 20 --json
```

Use graph commands for explicit link inspection.

```bash
cargo run -p vfs-cli --bin vfs-cli -- graph-neighborhood --center-path /Wiki/file.md --depth 1 --limit 100 --json
cargo run -p vfs-cli --bin vfs-cli -- graph-links --prefix /Wiki --limit 100 --json
cargo run -p vfs-cli --bin vfs-cli -- incoming-links --path /Wiki/file.md --limit 20 --json
cargo run -p vfs-cli --bin vfs-cli -- outgoing-links --path /Wiki/file.md --limit 20 --json
```

## Mirror Operations

Use `pull` and `push` for local mirror sync.

```bash
cargo run -p vfs-cli --bin vfs-cli -- pull --vault-path ./vault
cargo run -p vfs-cli --bin vfs-cli -- push --vault-path ./vault
```

## Skill Registry Preview

Skill Registry commands store `SKILL.md` packages as ordinary VFS wiki nodes under `/Wiki/skills`.
They do not add a canister schema or a dedicated registry API.
In v1, `skill import --source` accepts a local directory containing `SKILL.md`; remote GitHub fetch is not implemented.
See [`SKILL_REGISTRY.md`](SKILL_REGISTRY.md) for the manifest contract and audit behavior.

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --source ./skills/legal-review --id acme/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill list --prefix /Wiki/skills --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --fail-on error --json
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --output ./installed/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --skills-dir ~/.codex/skills
```

The registry writes:

- `/Wiki/skills/<publisher>/<name>/manifest.md`
- `/Wiki/skills/<publisher>/<name>/SKILL.md`
- `/Wiki/skills/<publisher>/<name>/provenance.md`
- `/Wiki/skills/<publisher>/<name>/evals.md`
- `/Sources/raw/skill-imports/<id>/<id>.md`

`manifest.md` uses Markdown with YAML frontmatter. The Rust CLI parses normal YAML for v1 validation. The Browser inspector uses a small read-only parser for the v1 display subset.
