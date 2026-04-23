#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_beam_improvement_slices.sh
# What: Run the BEAM improvement slices in the recommended order with fixed names.
# Why: Prompt-tuning checks should rerun the same question-type slices without ad hoc command drift.

if [[ $# -lt 3 ]]; then
  echo "usage: $0 <canister-id> <dataset-path> <output-root> [namespace] [extra args...]" >&2
  exit 1
fi

CANISTER_ID="$1"
DATASET_PATH="$2"
OUTPUT_ROOT="$3"
NAMESPACE="${4:-beam-full-reset}"

if [[ $# -ge 4 ]]; then
  shift 4
else
  shift 3
fi

EXTRA_ARGS=("$@")
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

SLICES=(
  preference-following
  information-extraction
  summarization
  multi-session-reasoning
  contradiction-resolution
  temporal-reasoning
)

for slice in "${SLICES[@]}"; do
  ARGS=(
    "$slice"
    "$CANISTER_ID"
    "$DATASET_PATH"
    "${OUTPUT_ROOT}/${slice}"
    "$NAMESPACE"
  )
  if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    ARGS+=("${EXTRA_ARGS[@]}")
  fi
  bash "${SCRIPT_DIR}/run_beam_grounded_slice.sh" "${ARGS[@]}"
done
