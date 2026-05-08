# Skill Registry

Skill Registry stores Agent Skills-compatible `SKILL.md` packages as ordinary wiki nodes.
It is a DB-backed skill knowledge base, not a GitHub or Vercel marketplace replacement.
GitHub is provenance/source context; the DB copy is the runtime source of truth.

Use it when a team wants skills to be searchable by task situation, review status, provenance, eval notes, and run evidence.
The product loop is:

```text
draft skill -> upsert -> find from task context -> inspect -> record run -> promote or deprecate
```

Access control is database-level.
Registry nodes follow the same `Owner`, `Writer`, and `Reader` roles as every other node in the database.
Use separate databases when different skill catalogs need different membership.

## Why Not Just A Skill Store

Vercel-style skill stores are useful as distribution shelves:

- publish or discover reusable skills
- install a skill into an agent environment
- treat GitHub or a package source as the main artifact history

Kinic Skill KB is for growing skills after teams start using them:

- search skills by task context, not only by package name
- keep `manifest.md`, `SKILL.md`, provenance, evals, and run evidence in one queryable DB
- record whether a skill actually helped a task under `/Sources/skill-runs/...`
- move skills through `draft`, `reviewed`, `promoted`, and `deprecated`
- share access with database roles instead of path-level ACL or marketplace visibility

GitHub is still the source and review history.
The DB copy is the operational record: what the team can find, trust, inspect, and improve from usage.

## Layout

Private or team skills live under `/Wiki/skills`:

```text
/Wiki/skills/<name>/manifest.md
/Wiki/skills/<name>/SKILL.md
/Wiki/skills/<name>/ingest.md
/Wiki/skills/<name>/provenance.md   # optional
/Wiki/skills/<name>/evals.md        # optional
```

Curated public skills use the same layout under `/Wiki/public-skills`:

```text
/Wiki/public-skills/<name>/manifest.md
/Wiki/public-skills/<name>/SKILL.md
/Wiki/public-skills/<name>/ingest.md
/Wiki/public-skills/<name>/provenance.md   # optional
/Wiki/public-skills/<name>/evals.md        # optional
```

`manifest.md` is the registry record.
`SKILL.md` is the Agent Skills entry file.
Package-local Markdown files referenced from `SKILL.md`, such as `ingest.md`, are stored with the package.
`provenance.md` and `evals.md` are optional long-form records.
Run evidence is stored as source nodes:

```text
/Sources/skill-runs/<name>/<timestamp>.md
```

## Manifest

`manifest.md` is Markdown with YAML frontmatter.
The Browser inspector parses a small read-only v1 display subset.

```yaml
---
kind: kinic.skill
schema_version: 1
id: legal-review
version: 0.1.0
entry: SKILL.md
title: Legal Review
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
  source: github.com/legal-review
  source_ref: abc123
---
# Skill Manifest
```

Required fields:

- `kind`: must be `kinic.skill`
- `schema_version`: must be `1`
- `id`: must use a single path-safe skill name
- `version`: skill package version
- `entry`: must be `SKILL.md` in v1
- `title`: display title, usually copied from `SKILL.md` frontmatter `metadata.title`

Optional fields:

- `summary`: one-line description used by `skill find`
- `tags`: search and grouping tags
- `use_cases`: task situations where the skill is useful
- `status`: `draft`, `reviewed`, `promoted`, or `deprecated`
- `replaces`: replaced skill ids
- `related`: related wiki or source paths
- `knowledge`: wiki paths the skill depends on
- `permissions`: declared expected access needs
- `provenance`: source, source revision, and upstream package metadata such as license

`manifest.md` is the Skill KB index and lifecycle record.
`SKILL.md` frontmatter is upstream package metadata input.
On `skill upsert`, empty manifest fields are filled from `SKILL.md`: `metadata.title` to `title`, `description` to `summary`, `metadata.category` to `tags`, and `license` to `provenance.license`.
Existing manifest values win.
`SKILL.md` `name` is an upstream runtime or display name and may differ from the DB skill id.

## CLI Usage

Use `database link` once, then run `skill` commands without repeating `--database-id`.
They are thin wrappers over normal VFS nodes and do not add canister schema or path-level ACL.
For the full first-run flow, see [`QUICKSTART_SKILL_KB.md`](QUICKSTART_SKILL_KB.md).

```bash
cargo run -p vfs-cli --bin vfs-cli -- database create team-skills
cargo run -p vfs-cli --bin vfs-cli -- database link team-skills
cargo run -p vfs-cli --bin vfs-cli -- skill upsert --source-dir ./skills/legal-review --id legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill find "review contract redlines"
cargo run -p vfs-cli --bin vfs-cli -- skill inspect legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill record-run legal-review --task "review vendor contract" --outcome success --notes-file ./notes.md
cargo run -p vfs-cli --bin vfs-cli -- skill set-status legal-review --status promoted
```

Share access with database member commands:

```bash
cargo run -p vfs-cli --bin vfs-cli -- database grant team-skills <principal> reader
cargo run -p vfs-cli --bin vfs-cli -- database grant team-skills <principal> writer
```

Status values are intentionally simple:

- `draft`: imported or experimental skill.
- `reviewed`: checked by the owning team.
- `promoted`: recommended for common use.
- `deprecated`: hidden from default `skill find`; include with `--include-deprecated`.

Run evidence under `/Sources/skill-runs/...` is the product differentiator.
It records what happened when a skill was used, including skill and manifest hashes, so teams can promote useful skills and retire weak ones.
`skill find` and `skill inspect` include `run_summary` with total runs, success/partial/fail counts, last use, and last outcome.
Old or invalid run evidence is ignored by `run_summary` but still appears in `recent_runs`.
`recorded_by: cli` is a v1 placeholder; principal-backed recording is deferred.
Path timestamps are millis IDs; frontmatter `*_at` timestamps are RFC3339.

`skill upsert` stores the package, not just the entry file.
It writes `SKILL.md`, `manifest.md`, optional `provenance.md` and `evals.md`, and direct package-local `.md` links from `SKILL.md`.
If `manifest.md` is missing, it is generated from `--id` plus `SKILL.md` frontmatter.
For example, `[ingest](ingest.md)` is stored as `/Wiki/skills/<name>/ingest.md`.
URLs, absolute paths, missing files, and files outside the package directory are ignored.
By default, upsert does not delete existing DB files.
Use `--prune` when the source package is the desired exact file set and stale package files should be removed.

Import uses existing package storage after fetching upstream files:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import github owner/repo:skills/legal-review --id legal-review --ref main --prune
```

GitHub import records `source`, `source_url`, and `revision` in manifest provenance.
Vercel and SkillHub are next-phase supply sources; this PR only exposes import commands that can complete successfully.

Improvement proposals are evidence-backed records, not automatic rewrites:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill propose-improvement legal-review \
  --runs /Sources/skill-runs/legal-review/123.md \
  --summary "Tighten missing-approval checks" \
  --diff-file ./proposal.diff
cargo run -p vfs-cli --bin vfs-cli -- skill approve-proposal legal-review /Wiki/skills/legal-review/improvement-proposals/123.md
```

`approve-proposal` marks the proposal approved. It does not apply the diff to `SKILL.md`; update the source package and run `skill upsert`.
Approval only accepts proposal nodes under the target skill's `improvement-proposals/` directory with matching proposal frontmatter.

## Example

The golden sample lives under [`../examples/skill-kb`](../examples/skill-kb):

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill upsert \
  --source-dir examples/skill-kb/skills/legal-review \
  --id legal-review \
  --prune
cargo run -p vfs-cli --bin vfs-cli -- skill find "contract review"
cargo run -p vfs-cli --bin vfs-cli -- skill record-run legal-review \
  --task "review vendor MSA redlines before counsel handoff" \
  --outcome success \
  --notes-file examples/skill-kb/runs/legal-review-success.md
```

## Browser

The wiki browser shows a read-only Skill card in the Inspector for registry paths.
When viewing `manifest.md`, the card is parsed from the current node.
When viewing package files such as `SKILL.md`, `ingest.md`, `provenance.md`, or `evals.md`, the browser reads the sibling `manifest.md` and displays the same skill metadata.
Registry access follows the selected database role.

## Agent Runtime

Agents can use Skill KB without shelling out to the CLI through the shared tool dispatcher:

```text
skill_find -> skill_inspect -> skill_read SKILL.md -> skill_read helper files -> skill_record_run
```

Discovery and read tools are read-only.
`skill_record_run` is a write tool and is not included in the read-only tool set.
All tools require `database_id` and use existing VFS reads, searches, and writes.
Agents should ignore `deprecated` skills by default, prefer `promoted` or `reviewed` candidates, and treat the read `SKILL.md` as task-local instruction.
Use the CLI for package operations such as `skill upsert`, import, proposal approval, and database linking.

## v1 Limits

- No path-level ACL.
- No signed release verification.
- No marketplace-wide hash pinning beyond per-run `skill_hash` and `manifest_hash`.
- No dependency resolution.
- No install-time execution permission enforcement.
- No dedicated Store UI.
- No automatic GitHub update monitoring.
- No automatic skill rewriting from evidence.
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
