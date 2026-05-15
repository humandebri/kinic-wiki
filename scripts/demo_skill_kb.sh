#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANISTER_ID="${CANISTER_ID:-}"
DATABASE_ID="${DATABASE_ID:-team-skills}"
LOCAL="${LOCAL:-0}"

if [[ -z "$CANISTER_ID" ]]; then
  echo "CANISTER_ID is required" >&2
  exit 1
fi

LOCAL_FLAG=()
if [[ "$LOCAL" == "1" || "$LOCAL" == "true" ]]; then
  LOCAL_FLAG=(--local)
fi

VFS=(cargo run -p kinic-vfs-cli --bin kinic-vfs-cli --)
SETUP=("${VFS[@]}")
if ((${#LOCAL_FLAG[@]})); then
  SETUP+=("${LOCAL_FLAG[@]}")
fi
SETUP+=(--canister-id "$CANISTER_ID")

cd "$ROOT_DIR"

CREATE_LOG="$(mktemp)"
trap 'rm -f "$CREATE_LOG"' EXIT
if ! "${SETUP[@]}" database create "$DATABASE_ID" >"$CREATE_LOG" 2>&1; then
  if grep -q "database already exists" "$CREATE_LOG"; then
    echo "database already exists: $DATABASE_ID"
  else
    cat "$CREATE_LOG" >&2
    exit 1
  fi
else
  cat "$CREATE_LOG"
fi
"${SETUP[@]}" database link "$DATABASE_ID"
"${VFS[@]}" database current
"${VFS[@]}" skill upsert \
  --source-dir examples/skill-kb/skills/legal-review \
  --id legal-review
"${VFS[@]}" skill find "contract review"
"${VFS[@]}" skill inspect legal-review
"${VFS[@]}" skill record-run legal-review \
  --task "review vendor MSA redlines before counsel handoff" \
  --outcome success \
  --notes-file examples/skill-kb/runs/legal-review-success.md
"${VFS[@]}" skill set-status legal-review --status promoted
"${VFS[@]}" skill inspect legal-review
