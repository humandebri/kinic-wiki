# Skill Knowledge Base Quickstart

This walkthrough turns a repo into a DB-linked skill catalog.
GitHub remains provenance and source history.
The Kinic DB copy is the searchable team record used by agents and CLI workflows.

This is different from a Vercel-style skill store.
A store helps distribute and install skills.
Skill KB helps a team find skills by task context, inspect provenance and evals, record run evidence, and promote or deprecate skills based on real usage.

## Prerequisites

- A deployed VFS canister.
- A principal with permission to create or write the target database.
- Local replica users should start and deploy the canister before running these commands.

Set the target canister:

```bash
export CANISTER_ID=<canister-id>
```

Use `--local` in each database setup command when targeting a local replica.

## 5 Minute Flow

Create and link a database.
`database create` is only needed the first time.
If `team-skills` already exists and you have access, start from `database link`.

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id "$CANISTER_ID" database create team-skills
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --canister-id "$CANISTER_ID" database link team-skills
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- database current
```

Upload the sample skill:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill upsert \
  --source-dir examples/skill-kb/skills/legal-review \
  --id legal-review \
  --prune
```

`skill upsert` uploads the package.
It stores `SKILL.md`, `manifest.md`, optional `provenance.md` and `evals.md`, plus direct package-local `.md` links from `SKILL.md`.
`--prune` removes stale package files already in the DB but no longer present in the source package.

Find and inspect it:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill find "contract review"
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill inspect legal-review
```

Record evidence from a real or demo run:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill record-run legal-review \
  --task "review vendor MSA redlines before counsel handoff" \
  --outcome success \
  --notes-file examples/skill-kb/runs/legal-review-success.md
```

Promote the skill after review:

```bash
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill set-status legal-review --status promoted
cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- skill inspect legal-review
```

## Team Operation

- `draft`: imported or experimental skill.
- `reviewed`: checked by the owning team.
- `promoted`: recommended for common use.
- `deprecated`: hidden from default `skill find`; use `--include-deprecated` to audit old skills.

Store private team skills under `/Wiki/skills`.
Store curated public catalog skills under `/Wiki/public-skills`.
Store usage evidence under `/Sources/skill-runs`.
Access follows database roles: `Owner`, `Writer`, and `Reader`.

## Troubleshooting

- Missing database link: run `database current`; if `database_id` is empty, run `database link <database-id>`.
- Permission denied: ask a database owner to grant `reader` for find/inspect or `writer` for upsert/record-run/set-status.
- Invalid manifest: check `kind`, `schema_version`, `id`, and `entry: SKILL.md`.
- Deprecated skill missing from search: rerun `skill find <query> --include-deprecated`.

## Demo Script

The scripted version uses the same sample:

```bash
CANISTER_ID=<canister-id> scripts/demo_skill_kb.sh
```

The script can be rerun with the same `DATABASE_ID`.
If the database already exists, it links the workspace and continues.

For local replica:

```bash
CANISTER_ID=<canister-id> LOCAL=1 scripts/demo_skill_kb.sh
```
