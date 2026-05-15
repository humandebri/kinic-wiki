#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/smoke/local_canister_archive_restore.sh
# What: Run archive/restore smoke against the project-local wiki canister.
# Why: SQLite byte archive flows need a deployed local canister check beyond Rust unit tests.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
IDS_FILE="${REPO_ROOT}/.icp/cache/mappings/local-wiki.ids.json"
REPLICA_HOST="${REPLICA_HOST:-http://127.0.0.1:8001}"

resolve_canister_id() {
  if [[ -n "${VFS_CANISTER_ID:-}" ]]; then
    printf '%s\n' "${VFS_CANISTER_ID}"
    return 0
  fi
  if [[ -f "${IDS_FILE}" ]]; then
    node -e '
      const fs = require("fs");
      const [filePath] = process.argv.slice(1);
      const ids = JSON.parse(fs.readFileSync(filePath, "utf8"));
      if (typeof ids.wiki !== "string" || ids.wiki.trim() === "") {
        process.exit(1);
      }
      process.stdout.write(ids.wiki);
    ' "${IDS_FILE}"
    return 0
  fi
  if [[ -n "${CANISTER_ID:-}" ]]; then
    printf '%s\n' "${CANISTER_ID}"
    return 0
  fi
  return 1
}

cd "${REPO_ROOT}"

if ! CANISTER_ID="$(resolve_canister_id)"; then
  echo "local wiki canister id not found; deploying wiki to local-wiki environment" >&2
  icp deploy -e local-wiki
  CANISTER_ID="$(resolve_canister_id)"
fi

export CANISTER_ID
export REPLICA_HOST

echo "running local canister archive/restore smoke against ${CANISTER_ID} at ${REPLICA_HOST}" >&2
cargo run -p kinic-vfs-cli --bin local_canister_archive_restore_smoke

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
INPUT_FILE="${TMP_DIR}/smoke.md"
ARCHIVE_FILE="${TMP_DIR}/archive.sqlite"
printf '# CLI Archive Smoke\n\nalpha archive restore smoke\n' > "$INPUT_FILE"

VFS=(cargo run -p kinic-vfs-cli --bin kinic-vfs-cli -- --local --canister-id "$CANISTER_ID")
CLI_DB="$("${VFS[@]}" database create)"
"${VFS[@]}" --database-id "$CLI_DB" write-node --path /Wiki/smoke.md --input "$INPUT_FILE"
"${VFS[@]}" database archive-export "$CLI_DB" --output "$ARCHIVE_FILE" --chunk-size 65536 --json
"${VFS[@]}" database archive-restore "$CLI_DB" --input "$ARCHIVE_FILE" --chunk-size 65536 --json
"${VFS[@]}" --database-id "$CLI_DB" read-node --path /Wiki/smoke.md --fields path,etag --json
