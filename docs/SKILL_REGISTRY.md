# Skill Registry

Kinic Skill Registry stores Agent Skills-compatible `SKILL.md` packages as ordinary wiki nodes.
It is a VFS-backed registry for private, team-approved, and curated public skills, with manifest metadata for knowledge links, provenance, permissions, and eval notes.

The registry does not add a canister schema or a dedicated registry API.
Access control is Principal-based and applies to `/Wiki/skills`, `/Wiki/public-skills`, and any other protected `/Wiki` path.
The curated public catalog lives under `/Wiki/public-skills`.
The default mode is `open`; enabling path policy switches the registry to `restricted`.

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

Or import the same package from GitHub:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --github acme/legal-skills --path skills/legal-review --ref main --id acme/legal-review
```

GitHub import uses `gh` authentication or `GITHUB_TOKEN` through the GitHub CLI.
The requested ref may be a branch, tag, or SHA, but the registry manifest stores the resolved commit SHA.
If `gh auth status -h github.com` fails, re-authenticate with `gh auth login -h github.com` before import or ingest.

If the source directory already contains `manifest.md`, import validates and preserves it.
The manifest `id` must match `--id`.
If `provenance.md` or `evals.md` exist in the source directory, import preserves those files too.

Inspect and audit it:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit acme/legal-review --fail-on error
```

Enable restricted access with a signed identity:

```bash
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy enable
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy grant <principal> Reader
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

Curated public skills use the same file layout under `/Wiki/public-skills`:

```text
/Wiki/public-skills/<publisher>/<name>/manifest.md
/Wiki/public-skills/<publisher>/<name>/SKILL.md
/Wiki/public-skills/<publisher>/<name>/provenance.md
/Wiki/public-skills/<publisher>/<name>/evals.md
```

Public catalog access follows the `/Wiki/public-skills` path policy.
Leave that policy disabled for a fully open catalog, or grant `Reader` to `2vxsx-fae` for anonymous reads after enabling it.

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

Import from GitHub:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill import --github acme/legal-skills --path skills/legal-review --ref main --id acme/legal-review
```

Update a GitHub-backed skill from its recorded source:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill update acme/legal-review --ref v0.2.0
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
Add `--lockfile` to write `skill.lock.json` with id, version, source path, manifest etag, and install time.
Local improvement commands expect the CLI-generated `skill.lock.json` shape.

### Personal skill index

Use `skills.index.toml` to manage the local preference set without installing every registry skill.
The index is local configuration; the registry remains ordinary VFS nodes.

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
Entries are listed by descending `priority`, then `id`.

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill index list --index ./skills.index.toml --json
cargo run -p vfs-cli --bin vfs-cli -- skill index inspect acme/legal-review --index ./skills.index.toml --json
cargo run -p vfs-cli --bin vfs-cli -- skill index install acme/legal-review --index ./skills.index.toml --output ./installed/legal-review --lockfile
cargo run -p vfs-cli --bin vfs-cli -- skill index install-enabled --index ./skills.index.toml --skills-dir ~/.codex/skills --lockfile --json
```

`skill index list` only parses the local file.
Install commands materialize selected skills on demand and reuse the existing lockfile format.

### Local improvement flow

Improve an installed skill locally without writing back to the registry:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill install acme/legal-review --output ./work/legal-review --lockfile
# Edit ./work/legal-review/SKILL.md, provenance.md, or evals.md.
cargo run -p vfs-cli --bin vfs-cli -- skill local diff ./work/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill local audit ./work/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill local install ./work/legal-review --skills-dir ~/.codex/skills
```

`skill local diff` uses `skill.lock.json` to compare local files with the registry source path and reports file-level `changed`, `added`, `missing`, or `unchanged` status.
`skill local audit` validates the local package shape and warns about missing optional provenance or eval notes.
`skill local install` copies the edited package to `<skills-dir>/<publisher>/<name>` and does not mutate the registry.
To share a local improvement, re-import explicitly with `skill import --source ./work/legal-review --id acme/legal-review`.

### Version history

Registry updates preserve the previous current package before overwriting it.
This applies to `skill import --source`, `skill import --github`, `skill update`, and `skill public promote`.
Initial imports and first public promotions do not create a version.

```text
/Wiki/skills/<publisher>/<name>/versions/<timestamp>-<manifest-etag>/manifest.md
/Wiki/skills/<publisher>/<name>/versions/<timestamp>-<manifest-etag>/SKILL.md
/Wiki/skills/<publisher>/<name>/versions/<timestamp>-<manifest-etag>/provenance.md
/Wiki/skills/<publisher>/<name>/versions/<timestamp>-<manifest-etag>/evals.md
```

List and inspect private registry versions:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill versions list acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill versions inspect acme/legal-review <version> --json
```

List and inspect public catalog versions:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill public versions list acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill public versions inspect acme/legal-review <version> --json
```

Version history is read-only in v1.
`skill install`, `skill local diff`, and `skill index install` continue to use the current package.

Promote an audit-clean private skill into the curated public catalog:

```bash
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill public promote acme/legal-review --json
```

`skill public promote` runs the private audit with `--fail-on warning` semantics before copying files.
Any manifest error, missing required file, missing raw source, incomplete provenance, dangerous instruction hint, or permission mismatch blocks promotion.

Read and install from the public catalog:

```bash
cargo run -p vfs-cli --bin vfs-cli -- skill public list --json
cargo run -p vfs-cli --bin vfs-cli -- skill public inspect acme/legal-review --json
cargo run -p vfs-cli --bin vfs-cli -- skill public install acme/legal-review --output ./installed/legal-review --lockfile --json
```

Public install writes the same package files to any output directory.
Its lockfile includes `catalog: "public"`, `source_path`, `manifest_etag`, and `installed_at`.
`--skills-dir` remains available and writes to `<skills-dir>/<publisher>/<name>`.

Revoke a public listing without deleting the private registry record:

```bash
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill public revoke acme/legal-review --json
```

Enable and manage Path policy. Use an Admin PEM for the initial enable and grants:

```bash
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy enable --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy policy --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy whoami --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy explain <principal> --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy list --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy grant <reader-principal> Reader
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem ./admin.pem skill policy grant <writer-principal> Writer
```

`VFS_IDENTITY_PEM` can be used instead of `--identity-pem`; explicit `--identity-pem` wins when both are set.
Roles are `Admin`, `Writer`, and `Reader`.
Use `skill policy policy --json` before enabling policy to confirm whether the registry is still `open`.
Use `skill policy whoami --json` to see the caller Principal, mode, roles, and `read/write/admin` capabilities.
Use `skill policy explain <principal> --json` as Admin to inspect a Principal's roles and capabilities.

`skill import --source` accepts a local directory containing `SKILL.md`.
`skill import --github` accepts `owner/repo` plus optional `--path`, or `owner/repo:path`.
Use either `--source` or `--github`, not both.
GitHub imports fetch `SKILL.md`, preserve optional `provenance.md` and `evals.md`, and pin manifest provenance to the resolved commit SHA.
`skill update` only works for skills whose manifest has a GitHub `provenance.source`.
Update automation, signed releases, and dependency resolution are deferred.

GitHub PR and issue evidence ingest is separate from Skill Registry import:

```bash
cargo run -p vfs-cli --bin vfs-cli -- github ingest pr acme/legal-skills#123
cargo run -p vfs-cli --bin vfs-cli -- github ingest issue acme/legal-skills#456
```

Evidence is stored under `/Sources/github/<owner>/<repo>/pulls/<number>.md` or `/Sources/github/<owner>/<repo>/issues/<number>.md`.

## GitHub Smoke

Use this after `gh auth status -h github.com` succeeds and a local canister is available.
The smoke writes only VFS nodes; it does not mirror a repository.

```bash
gh auth status -h github.com
cargo run -p vfs-cli --bin vfs-cli -- skill import --github <owner>/<repo> --path <skill-path> --ref <branch-or-tag> --id <publisher>/<name> --json
cargo run -p vfs-cli --bin vfs-cli -- skill inspect <publisher>/<name> --json
cargo run -p vfs-cli --bin vfs-cli -- skill audit <publisher>/<name> --json
cargo run -p vfs-cli --bin vfs-cli -- github ingest issue <owner>/<repo>#<number> --json
```

Expected result:

- `manifest.provenance.source_ref` is a 40-character commit SHA.
- `manifest.provenance.source_url` points at that SHA.
- `skill audit` has no GitHub pinning warning.
- GitHub evidence is written under `/Sources/github/...`.

## Access Control

Policy mode is `open` until `skill policy enable` is called.
In `open` mode, existing VFS behavior is unchanged.
In `restricted` mode:

- `Reader` can read, list, search, and install skills.
- `Writer` includes `Reader` and can write under `/Wiki/skills`.
- `Admin` includes `Writer` and can grant or revoke roles.
- Public catalog reads and writes under `/Wiki/public-skills` follow the `/Wiki/public-skills` policy when that namespace is restricted.

Canister filtering prevents unauthorized callers from seeing restricted paths through read, list, search, recent, graph, context, snapshot, or update-delta surfaces.
Non-skill VFS paths keep their existing behavior.
Path policy state is stored as `/System/path-policies.json` and is managed only through dedicated policy methods.
GitHub team or org sync is not part of v1; it can map to Principal roles later.
If a restricted call is made without a signed identity, the caller is anonymous and will normally need a role grant for principal `2vxsx-fae`; use a real identity for team registries.

Recommended team setup:

1. Run `skill policy enable` with the Admin PEM. The caller becomes the first `Admin`.
2. Ask viewers and publishers to run `skill policy whoami --json` with their PEM, or open the Browser and copy the Internet Identity Principal from the Inspector.
3. Grant `Reader` to read/install users and `Writer` to skill maintainers.
4. Have each user confirm Browser access on a `/Wiki/skills/...` page. The Inspector shows Principal, access mode, roles, and capabilities.

Writer check example:

```bash
VFS_IDENTITY_PEM=./writer.pem cargo run -p vfs-cli --bin vfs-cli -- skill policy whoami --json
```

Browser users authenticate with Internet Identity, so the Browser Principal can differ from a CLI PEM Principal.
Grant the Principal shown in the Browser Inspector when Browser access is required.
`knowledge_access` has been removed from the v1 manifest schema.
Protect private knowledge by enabling path policy on the knowledge path itself with the generic `enable_path_policy(path)` and grant/revoke APIs.
The `skill policy` command remains a convenience wrapper for `/Wiki/skills` only.
Use the generic path policy API for `/Wiki/public-skills`.

## Local Policy Smoke

Use this after deploying a local canister. The smoke is manual and is not part of normal CI.

```bash
export ADMIN_PEM=./admin.pem
export READER_PEM=./reader.pem
export WRITER_PEM=./writer.pem

cargo run -p vfs-cli --bin vfs-cli -- skill list --prefix /Wiki/skills --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$ADMIN_PEM" skill policy enable --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$ADMIN_PEM" skill policy whoami --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$READER_PEM" skill policy whoami --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$ADMIN_PEM" skill policy grant <reader-principal> Reader
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$ADMIN_PEM" skill policy grant <writer-principal> Writer
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$READER_PEM" skill list --prefix /Wiki/skills --json
cargo run -p vfs-cli --bin vfs-cli -- --identity-pem "$WRITER_PEM" skill import --source ./skills/legal-review --id acme/legal-review
cargo run -p vfs-cli --bin vfs-cli -- skill list --prefix /Wiki/skills --json
cargo run -p vfs-cli --bin vfs-cli -- read-node --path /System/path-policies.json --json
```

Expected result:

- Before `enable`, open mode permits normal registry listing.
- After `enable`, Admin `whoami` shows `can_admin: true`.
- Reader can list/install but cannot write.
- Writer can list and import/update skills but cannot edit policy entries.
- Public catalog promotion/revoke requires `Writer` or `Admin` on `/Wiki/public-skills` when that namespace is restricted.
- Anonymous callers can list, inspect, and install public skills only when `/Wiki/public-skills` is open or grants `Reader` to `2vxsx-fae`.
- Identity-free calls use anonymous principal `2vxsx-fae` and fail in restricted mode unless that Principal has a role.
- Direct reads of `/System/path-policies.json` fail; use `skill policy list --json` as Admin.

## JSON Output

`skill inspect --json`:

```json
{
  "id": "acme/legal-review",
  "base_path": "/Wiki/skills/acme/legal-review",
  "etag": "manifest-etag",
  "updated_at": 1710000000000,
  "raw_source": "./skills/legal-review",
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
  "files": ["manifest.md", "SKILL.md", "provenance.md", "evals.md"],
  "lockfile": null
}
```

`skill install --lockfile --json` sets `lockfile` to the written `skill.lock.json` path.

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
- raw import source exists
- provenance presence
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
| `provenance_incomplete` | Manifest provenance lacks `source` or `source_ref`. | Add both provenance fields. |
| `github_source_ref_not_pinned` | GitHub provenance uses a branch/tag-like ref instead of a commit SHA. | Re-import or update from GitHub so `source_ref` is pinned. |
| `github_source_url_missing` | GitHub provenance lacks a generated source URL. | Re-import or add `source_url`. |
| `raw_source_missing` | Import raw source record is absent. | Re-import or restore `/Sources/raw/skill-imports/...`. |
| `dangerous_instruction` | `SKILL.md` contains a known dangerous phrase. | Remove or rewrite the instruction. |
| `permission_network_mismatch` | `SKILL.md` references network access while `network` is false. | Remove network use or set `network: true`. |
| `permission_shell_mismatch` | `SKILL.md` references shell execution while `shell` is false. | Remove shell use or set `shell: true`. |
| `permission_file_read_mismatch` | `SKILL.md` references file reads while `file_read` is false. | Remove file access or set `file_read: true`. |
| `permission_secret_access_mismatch` | `SKILL.md` references secrets or environment access while shell permission is false. | Remove secret access or permit the needed execution surface. |

Structure warnings are `error`: `manifest_invalid`, `manifest_missing`, `file_missing`, `id_mismatch`, `publisher_mismatch`, and `entry_unsupported`.
Risk hints are `warning`: `dangerous_instruction`, provenance, raw source, knowledge protection, and permission mismatch codes.

## Browser

The wiki browser shows a read-only Skill card in the Inspector for skill registry paths.
When viewing `manifest.md`, the card is parsed from the current node.
When viewing `SKILL.md`, `provenance.md`, or `evals.md`, the browser reads the sibling `manifest.md` and displays the same skill metadata.
The top bar supports Internet Identity login.
The Inspector shows the current Principal, Policy mode, and roles for Skill Registry paths.
It includes a copy Principal button.
Admins see policy entries, grant/revoke controls, Principal validation, and role capability hints.
When a restricted registry rejects a Browser request, the error is shown as generic permission denial with a missing-role hint.

## v1 Limitations

- No signed release verification.
- No hash pinning.
- No dependency resolution.
- No install-time execution permission enforcement.
- No dedicated Store UI.
- No version restore, version install, or unified diff.
- No automatic GitHub update monitoring.
- No GitHub org/team policy sync.
- No implicit protected knowledge from skill manifests; protect knowledge paths with explicit path policy.

## Next Implementation Order

1. Add explicit dependency declarations and dependency health checks.
2. Add hash pinning and stronger provenance verification.
3. Add signed release verification for GitHub-backed skills.
4. Add a dedicated Browser registry index view.

## Validation

Run the standard checks after changing the registry:

```bash
cargo test -p vfs-cli --lib
cargo test --workspace
pnpm --dir wikibrowser test
pnpm --dir wikibrowser typecheck
```
