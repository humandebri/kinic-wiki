# Skill Registry

Skill Registry stores Agent Skills-compatible `SKILL.md` packages as ordinary wiki nodes.
It is a DB-backed skill knowledge base, not a GitHub or Vercel marketplace replacement.
GitHub is provenance/source context; the DB copy is the runtime source of truth.

Access control is database-level.
Registry nodes follow the same `Owner`, `Writer`, and `Reader` roles as every other node in the database.
Use separate databases when different skill catalogs need different membership.

## Layout

Private or team skills live under `/Wiki/skills`:

```text
/Wiki/skills/<publisher>/<name>/manifest.md
/Wiki/skills/<publisher>/<name>/SKILL.md
/Wiki/skills/<publisher>/<name>/provenance.md
/Wiki/skills/<publisher>/<name>/evals.md
```

Curated public skills use the same layout under `/Wiki/public-skills`:

```text
/Wiki/public-skills/<publisher>/<name>/manifest.md
/Wiki/public-skills/<publisher>/<name>/SKILL.md
/Wiki/public-skills/<publisher>/<name>/provenance.md
/Wiki/public-skills/<publisher>/<name>/evals.md
```

`manifest.md` is the registry record.
`SKILL.md` is the Agent Skills entry file.
`provenance.md` records source and review context.
`evals.md` records evaluation notes or benchmark results.
Run evidence is stored as source nodes:

```text
/Sources/skill-runs/<publisher>/<name>/<timestamp>.md
```

## Manifest

`manifest.md` is Markdown with YAML frontmatter.
The Browser inspector parses a small read-only v1 display subset.

```yaml
---
kind: kinic.skill
schema_version: 1
id: acme/legal-review
version: 0.1.0
publisher: acme
entry: SKILL.md
summary: Contract review workflow
tags:
  - legal
use_cases:
  - Review contract redlines
status: reviewed
replaces: []
related:
  - /Wiki/legal/contracts.md
knowledge:
  - /Wiki/legal/contracts.md
permissions:
  file_read: true
  network: false
  shell: false
provenance:
  source: github.com/acme/legal-review
  source_ref: abc123
---
# Skill Manifest
```

Required fields:

- `kind`: must be `kinic.skill`
- `schema_version`: must be `1`
- `id`: must use `publisher/name`
- `version`: skill package version
- `publisher`: must match the `id` publisher segment
- `entry`: must be `SKILL.md` in v1

Optional fields:

- `summary`: one-line description used by `skill find`
- `tags`: search and grouping tags
- `use_cases`: task situations where the skill is useful
- `status`: `draft`, `reviewed`, `promoted`, or `deprecated`
- `replaces`: replaced skill ids
- `related`: related wiki or source paths
- `knowledge`: wiki paths the skill depends on
- `permissions`: declared expected access needs
- `provenance`: source and source revision metadata

## CLI Usage

Use `database link` once, then run `skill` commands without repeating `--database-id`.
They are thin wrappers over normal VFS nodes and do not add canister schema or path-level ACL.

```bash
cargo run -p vfs-cli --bin vfs-cli -- database create team-skills
cargo run -p vfs-cli --bin vfs-cli -- database link team-skills
cargo run -p vfs-cli --bin vfs-cli -- skill upsert --source-dir ./skills/legal-review --id acme/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill find "review contract redlines"
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill record-run acme/legal-review --task "review vendor contract" --outcome success --notes-file ./notes.md
cargo run -p vfs-cli --bin vfs-cli -- skill set-status acme/legal-review --status promoted
```

Share access with database member commands:

```bash
cargo run -p vfs-cli --bin vfs-cli -- database grant team-skills <principal> reader
cargo run -p vfs-cli --bin vfs-cli -- database grant team-skills <principal> writer
```

## Browser

The wiki browser shows a read-only Skill card in the Inspector for registry paths.
When viewing `manifest.md`, the card is parsed from the current node.
When viewing `SKILL.md`, `provenance.md`, or `evals.md`, the browser reads the sibling `manifest.md` and displays the same skill metadata.
Registry access follows the selected database role.

## v1 Limits

- No path-level ACL.
- No signed release verification.
- No hash pinning.
- No dependency resolution.
- No install-time execution permission enforcement.
- No dedicated Store UI.
- No automatic GitHub update monitoring.
- No GitHub org/team policy sync.
- No skill install command.
- No implicit protected knowledge from skill manifests; use separate databases for different access boundaries.

## Validation

Run the standard checks after changing registry behavior:

```bash
cargo test --workspace
pnpm --dir wikibrowser test
pnpm --dir wikibrowser typecheck
```
