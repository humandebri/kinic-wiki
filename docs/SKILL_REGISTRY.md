# Skill Registry

Kinic Skill Registry stores Agent Skills-compatible `SKILL.md` packages as ordinary wiki nodes.
It is a VFS-backed registry for private and team-approved skills, with manifest metadata for knowledge links, provenance, permissions, and eval notes.

The registry does not add a canister schema or a dedicated registry API.

## Quick Start

Create a local skill directory with at least `SKILL.md`:

```text
skills/legal-review/
└── SKILL.md
```

Minimal `SKILL.md`:

```md
# Legal Review

Use the linked wiki knowledge to review contract risk.
Do not invent citations.
```

Import it:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --source ./skills/legal-review --id acme/legal-review
```

If the source directory already contains `manifest.md`, import validates and preserves it.
The manifest `id` must match `--id`.
If `provenance.md` or `evals.md` exist in the source directory, import preserves those files too.

Inspect and audit it:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --fail-on error
```

## Layout

Each skill uses `publisher/name` identity and lives under `/Wiki/skills`:

```text
/Wiki/skills/<publisher>/<name>/manifest.md
/Wiki/skills/<publisher>/<name>/SKILL.md
/Wiki/skills/<publisher>/<name>/provenance.md
/Wiki/skills/<publisher>/<name>/evals.md
/Sources/raw/skill-imports/<publisher-name>/<publisher-name>.md
```

`manifest.md` is the registry record.
`SKILL.md` is the Agent Skills entry file.
`provenance.md` records source and review context.
`evals.md` records evaluation notes or benchmark results.
The raw source node records the import event.

## Manifest

`manifest.md` is Markdown with YAML frontmatter.
The Rust CLI validates normal YAML for schema version 1.
The Browser inspector uses a small read-only parser for the v1 display subset.

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

- `knowledge`: wiki paths the skill depends on
- `permissions`: declared expected access needs
- `provenance`: source and source revision metadata

## CLI

Import from a local skill directory:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --source ./skills/legal-review --id acme/legal-review
```

Inspect a registry record:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
```

List registered skills:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill list --prefix /Wiki/skills --json
```

Audit a skill:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --json
```

Install files into a local directory:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --output ./installed/legal-review
```

Install into a skills directory:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --skills-dir ~/.codex/skills
```

`--skills-dir` writes to `<skills-dir>/<publisher>/<name>`.
Use either `--output` or `--skills-dir`, not both.

In v1, `skill import --source` accepts only a local directory containing `SKILL.md`.
Remote GitHub fetch, update automation, signed releases, and dependency resolution are deferred.

## JSON Output

`skill inspect --json`:

```json
{
  "id": "acme/legal-review",
  "base_path": "/Wiki/skills/acme/legal-review",
  "manifest": {
    "kind": "kinic.skill",
    "schema_version": 1,
    "id": "acme/legal-review",
    "version": "0.1.0",
    "publisher": "acme",
    "entry": "SKILL.md",
    "knowledge": [],
    "permissions": {
      "file_read": true,
      "network": false,
      "shell": false
    },
    "provenance": {
      "source": "./skills/legal-review",
      "source_ref": "local"
    }
  },
  "files": {
    "SKILL.md": true,
    "evals.md": true,
    "provenance.md": true
  },
  "warnings": []
}
```

`skill audit --json`:

```json
{
  "id": "acme/legal-review",
  "ok": false,
  "warnings": [
    {
      "code": "permission_network_mismatch",
      "severity": "warning",
      "message": "SKILL.md references network access but permissions.network is false"
    }
  ]
}
```

`severity` can be `error`, `warning`, or `info`.
Current v1 audit uses `error` for invalid package structure and `warning` for risky instructions or permission mismatch hints.
`ok` is true only when the audit emits no warnings at any severity.

`skill list --json`:

```json
[
  {
    "id": "acme/legal-review",
    "version": "0.1.0",
    "publisher": "acme",
    "path": "/Wiki/skills/acme/legal-review/manifest.md"
  }
]
```

`skill install --json`:

```json
{
  "id": "acme/legal-review",
  "output": "./installed/legal-review",
  "files": ["manifest.md", "SKILL.md", "provenance.md", "evals.md"]
}
```

## Audit

`skill audit` emits warnings and does not block import or install.
`--fail-on error` returns a non-zero exit when any `error` is present.
`--fail-on warning` returns a non-zero exit when any `warning` or `error` is present.
`--fail-on` changes only the command result; it does not change the JSON payload.

It checks:

- required manifest fields
- `id` and `publisher` consistency
- `entry: SKILL.md`
- `knowledge` paths stay under `/Wiki` and exist
- dangerous instruction phrases
- permission mismatch hints for `network`, `shell`, and `file_read`

Permission checks are static hints.
They do not prove a skill is safe.

### Warning Codes

| Code | Meaning | Typical fix |
| --- | --- | --- |
| `manifest_invalid` | `manifest.md` exists but cannot be parsed as v1. | Fix YAML frontmatter and required fields. |
| `manifest_missing` | Registry record has no `manifest.md`. | Re-import or write the manifest. |
| `file_missing` | One of `SKILL.md`, `provenance.md`, or `evals.md` is missing. | Add the missing file. |
| `id_mismatch` | Manifest `id` differs from the inspected id. | Make manifest `id` match `publisher/name`. |
| `publisher_mismatch` | Manifest publisher differs from the id prefix. | Align `publisher` with the id prefix. |
| `entry_unsupported` | Manifest `entry` is not `SKILL.md`. | Use `entry: SKILL.md` in v1. |
| `knowledge_outside_wiki` | A knowledge path is outside `/Wiki`. | Move the path under `/Wiki` or remove it. |
| `knowledge_missing` | A declared knowledge path does not exist. | Create the wiki page or remove the reference. |
| `dangerous_instruction` | `SKILL.md` contains a known dangerous phrase. | Remove or rewrite the instruction. |
| `permission_network_mismatch` | `SKILL.md` references network access while `network` is false. | Remove network use or set `network: true`. |
| `permission_shell_mismatch` | `SKILL.md` references shell execution while `shell` is false. | Remove shell use or set `shell: true`. |
| `permission_file_read_mismatch` | `SKILL.md` references file reads while `file_read` is false. | Remove file access or set `file_read: true`. |

Structure warnings are `error`: `manifest_invalid`, `manifest_missing`, `file_missing`, `id_mismatch`, `publisher_mismatch`, and `entry_unsupported`.
Risk hints are `warning`: `dangerous_instruction` and permission mismatch codes.

## Browser

The wiki browser shows a read-only Skill card in the Inspector for skill registry paths.
When viewing `manifest.md`, the card is parsed from the current node.
When viewing `SKILL.md`, `provenance.md`, or `evals.md`, the browser reads the sibling `manifest.md` and displays the same skill metadata.

## v1 Limitations

- No remote GitHub fetch.
- No signed release verification.
- No lockfile or hash pinning.
- No dependency resolution.
- No install-time execution permission enforcement.
- No dedicated Store UI.
- Browser support is read-only.

## Next Implementation Order

1. Add explicit dependency declarations and dependency health checks.
2. Add lockfile, hash pinning, and provenance verification.
3. Add remote GitHub fetch after trust policy is defined.
4. Add a dedicated Browser registry index view.

## Validation

Run the standard checks after changing the registry:

```bash
cargo test -p vfs-cli --lib
cargo test --workspace
pnpm --dir wikibrowser test
pnpm --dir wikibrowser typecheck
```
