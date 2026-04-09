#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_all_vfs_benchmarks.sh
# What: Run repo-local checks followed by external VFS benchmark wrappers.
# Why: VFS validation should have a single entrypoint that keeps the execution order consistent.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
EXPECTED_POCKET_IC_VERSION="pocket-ic-server 10.0.0"

run_step() {
  local label="$1"
  shift
  echo "[run-all] ${label}"
  "$@"
}

resolve_runtime_bin() {
  local -a candidates=()

  candidates+=(
    "${REPO_ROOT}/.canbench/pocket-ic"
    "${REPO_ROOT}/pocket-ic"
  )
  if [[ -n "${POCKET_IC_BIN:-}" ]]; then
    candidates+=("${POCKET_IC_BIN}")
  fi

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -x "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

run_step "cargo test --workspace" cargo test --workspace
run_step "plugins/kinic-wiki npm run check" bash -lc "cd \"${REPO_ROOT}/plugins/kinic-wiki\" && npm run check"
run_step "build canbench wasm" bash "${REPO_ROOT}/scripts/build-wiki-canister-canbench.sh"

runtime_available=0
RUNTIME_BIN=""
if RUNTIME_BIN="$(resolve_runtime_bin)"; then
  runtime_available=1
fi

if [[ -x "${REPO_ROOT}/.canbench-tools/bin/canbench" ]] || command -v canbench >/dev/null 2>&1; then
  if [[ "${runtime_available}" -eq 1 ]]; then
    runtime_version="$("${RUNTIME_BIN}" --version)"
    if [[ "${runtime_version}" == "${EXPECTED_POCKET_IC_VERSION}" ]]; then
      run_step "run canbench guard" bash "${REPO_ROOT}/scripts/run_canbench_guard.sh"
    else
      echo "[run-all] skipping canbench guard: runtime version mismatch (${runtime_version})"
    fi
  else
    echo "[run-all] skipping canbench guard: runtime not found"
  fi
else
  echo "[run-all] skipping canbench guard: canbench binary not found"
fi

run_step "fio VFS benchmarks" bash "${REPO_ROOT}/scripts/bench/run_fio_vfs.sh"
run_step "smallfile VFS benchmarks" bash "${REPO_ROOT}/scripts/bench/run_smallfile_vfs.sh"
run_step "SQLite speedtest1 benchmarks" bash "${REPO_ROOT}/scripts/bench/run_sqlite_speedtest1.sh"
run_step "SQLite commit latency benchmarks" bash "${REPO_ROOT}/scripts/bench/run_sqlite_commit_latency_vfs.sh"
