# Kinic Skill Registry

Kinic Skill Registry stores Agent Skills-compatible `SKILL.md` packages in the existing VFS wiki.
The first version targets private and team registries, not a public marketplace.

## Position

- Product claim: verifiable skills with attached knowledge.
- Primary users: agent-heavy developers, small teams, and expert domains where evidence matters.
- Non-goals: prompt marketplace, payment flow, MCP registry replacement, public submission queue.

## VFS Layout

- `/Wiki/skills/<publisher>/<name>/manifest.md`
- `/Wiki/skills/<publisher>/<name>/SKILL.md`
- `/Wiki/skills/<publisher>/<name>/provenance.md`
- `/Wiki/skills/<publisher>/<name>/evals.md`
- `/Sources/raw/skill-imports/<id>/<id>.md`

`manifest.md` uses Markdown with YAML frontmatter:

```yaml
---
kind: kinic.skill
schema_version: 1
id: acme/legal-review
version: 0.1.0
publisher: acme
entry: SKILL.md
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
```

## Current Surface

- CLI: `skill import`, `skill inspect`, `skill list`, `skill audit`, `skill install`.
- Browser: read-only Inspector card for `kind: kinic.skill` manifests under `/Wiki/skills`.
- Storage: existing VFS nodes only. No canister migration or dedicated registry API.
- Import: v1 accepts only a local directory containing `SKILL.md`; remote GitHub fetch is deferred.
- Parsing: Rust CLI validates normal YAML frontmatter. Browser uses a small v1 subset parser for display only.

## Deferred

- Signed releases.
- Dependency resolution.
- Remote update automation.
- Payment and revenue share.
- Public Store UI.
