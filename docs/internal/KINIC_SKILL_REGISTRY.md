# Kinic Skill Registry

Kinic Skill Registry stores Agent Skills-compatible `SKILL.md` packages in the existing VFS wiki.
The first version targets private and team registries, not a public marketplace.

## Position

- Product claim: verifiable skills with attached knowledge.
- Primary users: agent-heavy developers, small teams, and expert domains where evidence matters.
- Non-goals: prompt marketplace, payment flow, MCP registry replacement, public submission queue.

## VFS Layout

- `/Wiki/skills/<name>/manifest.md`
- `/Wiki/skills/<name>/SKILL.md`
- `/Wiki/skills/<name>/<package-local>.md`
- `/Wiki/skills/<name>/provenance.md` optional
- `/Wiki/skills/<name>/evals.md` optional
- `/Sources/raw/skill-imports/<id>/<id>.md`

`manifest.md` uses Markdown with YAML frontmatter:

```yaml
---
kind: kinic.skill
schema_version: 1
id: legal-review
version: 0.1.0
entry: SKILL.md
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
```

## Current Surface

- CLI: `skill upsert`, `skill find`, `skill inspect`, `skill record-run`, and `skill set-status` with explicit `database_id`.
- Browser: dedicated `/skills/<database-id>` catalog and operations UI. The wiki browser treats registry paths as ordinary wiki nodes.
- Storage: existing VFS nodes only. No canister migration or dedicated registry API.
- Import: `skill upsert` writes package files as wiki nodes, including direct package-local `.md` links from `SKILL.md`; GitHub remains provenance/source context.
- Parsing: CLI parses YAML frontmatter for updates. Browser uses a small v1 subset parser for display only.
- Access control: Skill Registry nodes follow database `Owner`, `Writer`, and `Reader` roles.
- Stabilization: CLI tests cover node-backed find/inspect/status/run behavior, and Browser tests cover manifest display parsing.

## Deferred

- Signed releases.
- Dependency resolution.
- Remote update automation.
- Payment and revenue share.
- Public Store UI.
- GitHub org/team role sync.
