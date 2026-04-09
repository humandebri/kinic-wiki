#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_canister_vfs_latency.sh
# What: Run single-update latency benchmarks against a deployed canister.
# Why: We need a canister-side analogue to durable mutation latency without canbench.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

require_command "cargo" "Rust toolchain is required to build the vfs_bench client binary."
require_command "node" "Node.js is required to materialize JSON summaries."

REPLICA_HOST="${REPLICA_HOST:-}"
CANISTER_ID="${CANISTER_ID:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --replica-host) REPLICA_HOST="$2"; shift 2 ;;
    --canister-id) CANISTER_ID="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done
if [[ -z "${REPLICA_HOST}" || -z "${CANISTER_ID}" ]]; then
  echo "usage: REPLICA_HOST=... CANISTER_ID=... bash scripts/bench/run_canister_vfs_latency.sh" >&2
  exit 1
fi

RESULT_DIR="$(bench_results_dir "canister_vfs_latency")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
BENCH_BIN="$(bench_repo_root)/target/debug/vfs_bench"
write_summary_header "${SUMMARY_FILE}" "canister_vfs_latency"
write_environment_json "${ENVIRONMENT_FILE}"
augment_environment_json "${ENVIRONMENT_FILE}" "${REPLICA_HOST}" "${CANISTER_ID}" "ic-agent"

bench_log "building vfs_bench binary"
cargo build -p wiki-cli --bin vfs_bench >/dev/null

cat > "${CONFIG_FILE}" <<EOF
{
  "tool": "canister_vfs_latency",
  "replica_host": "${REPLICA_HOST}",
  "canister_id": "${CANISTER_ID}",
  "benchmark_transport": "ic-agent",
  "iterations": ${LATENCY_ITERATIONS:-1000},
  "warmup_iterations": ${LATENCY_WARMUP_ITERATIONS:-20},
  "scenarios": [
    { "scenario": "write_node_single_1k", "operation": "write-node", "payload_size_bytes": 1024 },
    { "scenario": "write_node_single_4k", "operation": "write-node", "payload_size_bytes": 4096 },
    { "scenario": "append_node_single_1k", "operation": "append-node", "payload_size_bytes": 1024 },
    { "scenario": "append_node_single_4k", "operation": "append-node", "payload_size_bytes": 4096 }
  ]
}
EOF

append_summary() {
  local raw_json="$1"
  node -e '
    const fs = require("fs");
    const [rawJson] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(rawJson, "utf8"));
    const lines = [
      `scenario=${data.benchmark_name}`,
      `operation=${data.operation}`,
      `payload_size_bytes=${data.payload_size_bytes}`,
      `iterations=${data.iterations}`,
      `warmup_iterations=${data.warmup_iterations}`,
      `request_count=${data.request_count}`,
      `total_seconds=${data.total_seconds}`,
      `avg_latency_us=${data.avg_latency_us}`,
      `p50_latency_us=${data.p50_latency_us}`,
      `p95_latency_us=${data.p95_latency_us}`,
      `p99_latency_us=${data.p99_latency_us}`,
      `raw_json=${rawJson}`,
      ""
    ];
    console.log(lines.join("\n"));
  ' "${raw_json}" >> "${SUMMARY_FILE}"
}

node -e '
  const fs = require("fs");
  const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
  for (const scenario of data.scenarios) {
    console.log([scenario.scenario, scenario.operation, scenario.payload_size_bytes].join("|"));
  }
' "${CONFIG_FILE}" | while IFS='|' read -r scenario operation payload_size; do
  raw_json="${RAW_DIR}/${scenario}.json"
  prefix="/Wiki/bench/deployed/${RUN_TIMESTAMP}/latency/${scenario}"
  bench_log "canister latency ${scenario}"
  "${BENCH_BIN}" latency \
    --output-json "${raw_json}" \
    --benchmark-name "${scenario}" \
    --replica-host "${REPLICA_HOST}" \
    --canister-id "${CANISTER_ID}" \
    --prefix "${prefix}" \
    --payload-size-bytes "${payload_size}" \
    --iterations "${LATENCY_ITERATIONS:-1000}" \
    --warmup-iterations "${LATENCY_WARMUP_ITERATIONS:-20}" \
    --operation "${operation}"
  append_summary "${raw_json}"
done

bench_log "canister VFS latency results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
