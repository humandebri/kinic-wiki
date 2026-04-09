#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_canister_vfs_workload.sh
# What: Run deployed-canister workload benchmarks that mirror smallfile-style VFS inputs.
# Why: We need canister-side FS-shaped measurements without relying on canbench.

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
  echo "usage: REPLICA_HOST=... CANISTER_ID=... bash scripts/bench/run_canister_vfs_workload.sh" >&2
  exit 1
fi

RESULT_DIR="$(bench_results_dir "canister_vfs_workload")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
BENCH_BIN="$(bench_repo_root)/target/debug/vfs_bench"
write_summary_header "${SUMMARY_FILE}" "canister_vfs_workload"
write_environment_json "${ENVIRONMENT_FILE}"
augment_environment_json "${ENVIRONMENT_FILE}" "${REPLICA_HOST}" "${CANISTER_ID}" "ic-agent"

bench_log "building vfs_bench binary"
cargo build -p wiki-cli --bin vfs_bench >/dev/null

node -e '
  const fs = require("fs");
  const [configFile, replicaHost, canisterId] = process.argv.slice(1);
  const parseList = (value, fallback) => {
    if (!value || value.trim().length === 0) return fallback;
    return value.split(",").map(item => item.trim()).filter(Boolean);
  };
  const parseNumberList = (value, fallback) => parseList(value, fallback.map(String)).map(Number);
  const operations = parseList(
    process.env.WORKLOAD_OPERATIONS,
    ["create", "rename_same_dir", "rename_cross_dir", "delete", "read_single", "list_prefix"]
  );
  const directoryShapes = parseList(
    process.env.WORKLOAD_DIRECTORY_SHAPES,
    ["flat", "fanout100x100"]
  );
  const temperatures = parseList(
    process.env.WORKLOAD_TEMPERATURES,
    ["cold_seeded", "warm_repeat"]
  );
  const fileCounts = parseNumberList(process.env.WORKLOAD_FILE_COUNTS, [100, 1000, 10000, 100000]);
  const payloadSizes = parseNumberList(process.env.WORKLOAD_PAYLOAD_SIZES, [1024, 4096]);
  const clients = parseNumberList(process.env.WORKLOAD_CONCURRENT_CLIENTS, [1, 4, 8]);
  const scenarios = [];
  for (const operation of operations) for (const shape of directoryShapes)
    for (const temperature of temperatures) for (const fileCount of fileCounts)
      for (const payloadSize of payloadSizes) for (const concurrentClients of clients) {
        scenarios.push({
          scenario: `${operation}_${shape}_${temperature}_n${fileCount}_p${payloadSize}_c${concurrentClients}`,
          operation,
          directory_shape: shape,
          temperature,
          file_count: fileCount,
          payload_size_bytes: payloadSize,
          concurrent_clients: concurrentClients,
          iterations: operation === "list_prefix" ? 100 : fileCount,
          warmup_iterations: temperature === "warm_repeat" ? 3 : 0
        });
      }
  const payload = {
    tool: "canister_vfs_workload",
    replica_host: replicaHost,
    canister_id: canisterId,
    benchmark_transport: "ic-agent",
    operations,
    directory_shapes: directoryShapes,
    temperatures,
    file_counts: fileCounts,
    payload_size_bytes: payloadSizes,
    concurrent_clients: clients,
    scenarios
  };
  fs.writeFileSync(configFile, JSON.stringify(payload, null, 2) + "\n");
' "${CONFIG_FILE}" "${REPLICA_HOST}" "${CANISTER_ID}"

append_summary() {
  local raw_json="$1"
  node -e '
    const fs = require("fs");
    const [rawJson] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(rawJson, "utf8"));
    const lines = [
      `scenario=${data.benchmark_name}`,
      `operation=${data.operation}`,
      `temperature=${data.temperature}`,
      `directory_shape=${data.directory_shape}`,
      `file_count=${data.file_count}`,
      `payload_size_bytes=${data.payload_size_bytes}`,
      `concurrent_clients=${data.concurrent_clients}`,
      `iterations=${data.iterations}`,
      `warmup_iterations=${data.warmup_iterations}`,
      `request_count=${data.request_count}`,
      `seed_seconds=${data.seed_seconds}`,
      `wall_seconds=${data.wall_seconds}`,
      `total_seconds=${data.total_seconds}`,
      `ops_per_sec=${data.ops_per_sec}`,
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
    console.log([
      scenario.scenario,
      scenario.operation,
      scenario.directory_shape,
      scenario.temperature,
      scenario.file_count,
      scenario.payload_size_bytes,
      scenario.concurrent_clients,
      scenario.iterations,
      scenario.warmup_iterations
    ].join("|"));
  }
' "${CONFIG_FILE}" | while IFS='|' read -r scenario operation directory_shape temperature file_count payload_size concurrent_clients iterations warmup_iterations; do
  raw_json="${RAW_DIR}/${scenario}.json"
  prefix="/Wiki/bench/deployed/${RUN_TIMESTAMP}/workload/${scenario}"
  cli_operation="${operation//_/-}"
  cli_temperature="${temperature//_/-}"
  bench_log "canister workload ${scenario}"
  "${BENCH_BIN}" workload \
    --output-json "${raw_json}" \
    --benchmark-name "${scenario}" \
    --replica-host "${REPLICA_HOST}" \
    --canister-id "${CANISTER_ID}" \
    --prefix "${prefix}" \
    --payload-size-bytes "${payload_size}" \
    --file-count "${file_count}" \
    --directory-shape "${directory_shape}" \
    --concurrent-clients "${concurrent_clients}" \
    --iterations "${iterations}" \
    --warmup-iterations "${warmup_iterations}" \
    --temperature "${cli_temperature}" \
    --operation "${cli_operation}"
  append_summary "${raw_json}"
done

bench_log "canister VFS workload results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
