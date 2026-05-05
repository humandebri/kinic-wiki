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
Curated public skills live under `/Wiki/public-skills` and follow that path's policy when restricted.
They do not add a canister schema or a dedicated registry API.
`skill import` accepts either a local directory or a GitHub source. GitHub is only an import source; VFS remains the registry source of truth.
See [`SKILL_REGISTRY.md`](SKILL_REGISTRY.md) for the manifest contract and audit behavior.

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --source ./skills/legal-review --id acme/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill import --github acme/legal-skills --path skills/legal-review --ref main --id acme/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill update acme/legal-review --ref v0.2.0
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill list --prefix /Wiki/skills --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --fail-on error --json
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --output ./installed/legal-review --lockfile
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --skills-dir ~/.codex/skills
cargo run -p vfs-cli --bin vfs-cli -- skill versions list acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill versions inspect acme/legal-review <version> --json
cargo run -p vfs-cli --bin vfs-cli -- skill index list --index ./skills.index.toml --json
cargo run -p vfs-cli --bin vfs-cli -- skill index inspect acme/legal-review --index ./skills.index.toml --json
cargo run -p vfs-cli --bin vfs-cli -- skill index install acme/legal-review --index ./skills.index.toml --output ./installed/legal-review --lockfile
cargo run -p vfs-cli --bin vfs-cli -- skill index install-enabled --index ./skills.index.toml --skills-dir ~/.codex/skills --lockfile --json
cargo run -p vfs-cli --bin vfs-cli -- skill local diff ./installed/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill local audit ./installed/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill local install ./installed/legal-review --skills-dir ~/.codex/skills
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill public promote acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill public list --json
cargo run -p vfs-cli --bin vfs-cli -- skill public inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill public install acme/legal-review --output ./installed/legal-review --lockfile --json
cargo run -p vfs-cli --bin vfs-cli -- skill public versions list acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill public versions inspect acme/legal-review <version> --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill public revoke acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill policy enable --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill policy policy --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill policy explain <principal> --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./identity.pem skill policy grant <principal> Reader
cargo run -p vfs-cli --bin vfs-cli -- github ingest pr acme/legal-skills#123
cargo run -p vfs-cli --bin vfs-cli -- github ingest issue acme/legal-skills#456
```

The registry writes:

- `/Wiki/skills/<publisher>/<name>/manifest.md`
- `/Wiki/skills/<publisher>/<name>/SKILL.md`
- `/Wiki/skills/<publisher>/<name>/provenance.md`
- `/Wiki/skills/<publisher>/<name>/evals.md`
- `/Sources/raw/skill-imports/<id>/<id>.md`

Public promotion copies the same four package files to `/Wiki/public-skills/<publisher>/<name>/`.
Promotion requires an Admin identity and a clean private audit with warning-level failures enabled.
Public revoke removes only the public package files and keeps the private registry record.
Public install writes to any `--output` directory; `--skills-dir` is retained for compatibility.
Import, GitHub update, and public promotion save the previous current package under `versions/<timestamp>-<etag>/` before overwriting it.
Local improvement commands read an installed skill directory, compare it to the lockfile source, audit local edits, and reinstall the edited copy into a local skills directory. They do not write to the registry.

Personal skill indexes are local TOML preference files. They do not mutate the registry.
The default path is `./skills.index.toml`.

```toml
version = 1

[[skills]]
id = "acme/legal-review"
catalog = "private"
enabled = true
priority = 100
```

`catalog` is `private` for `/Wiki/skills` or `public` for `/Wiki/public-skills`.
If omitted, `catalog` defaults to `private`, `enabled` defaults to `true`, and `priority` defaults to `0`.
`skill index list` only parses the local index.
`skill index install` and `skill index install-enabled` materialize selected skills on demand using the existing install behavior.

`manifest.md` uses Markdown with YAML frontmatter. The Rust CLI parses normal YAML for v1 validation. The Browser inspector uses a small read-only parser for the v1 display subset.
GitHub imports resolve `--ref` to a commit SHA and store it in `provenance.source_ref`.
GitHub PR and issue ingest writes source evidence under `/Sources/github/...`; it does not mirror repositories.
GitHub commands require `gh auth status -h github.com` to succeed; use `gh auth login -h github.com` if authentication is missing or invalid.
Path policy is Principal-based, defaults to `open`, and becomes `restricted` after policy enable.
`skill policy` is the Skill Store wrapper for the generic `/Wiki/skills` path policy.
Use generic path policy APIs for `/Wiki/public-skills`; restricted public catalogs require Reader for reads and Writer/Admin for writes.
`VFS_IDENTITY_PEM` can be used instead of `--identity-pem` for signed policy and write calls; explicit `--identity-pem` wins when both are set.
`skill policy whoami --json` reports the caller Principal, mode, roles, and `read/write/admin` capabilities.
`skill policy explain <principal> --json` reports a Principal's roles and capabilities for Admin review.
Without either identity option, restricted path policy calls use anonymous principal `2vxsx-fae`.
