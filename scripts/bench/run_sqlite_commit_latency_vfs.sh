#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_sqlite_commit_latency_vfs.sh
# What: Run single-row SQLite commit latency scenarios and persist raw/config/summary outputs.
# Why: VFS durability cost is dominated by commit latency, not just mixed speedtest1 throughput.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

require_command "cc" "A C compiler is required to build the SQLite commit latency helper."
require_command "sqlite3" "sqlite3 is required to link and execute the commit latency helper."
require_command "node" "Node.js is required to summarize commit latency output."

RESULT_DIR="$(bench_results_dir "sqlite_commit_latency")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
WORK_DIR="$(bench_work_dir "sqlite")"
BIN_DIR="${WORK_DIR}/bin"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
HELPER_SRC="${SCRIPT_DIR}/sqlite_commit_latency.c"
HELPER_BIN="${BIN_DIR}/sqlite_commit_latency"
write_summary_header "${SUMMARY_FILE}" "sqlite_commit_latency"
write_environment_json "${ENVIRONMENT_FILE}"
mkdir -p "${BIN_DIR}"

cat > "${CONFIG_FILE}" <<'EOF'
{
  "tool": "sqlite_commit_latency",
  "benchmark_role": "primary durability-sensitive SQLite benchmark",
  "payload_size": 1024,
  "iterations": [1000, 10000],
  "journal_modes": ["WAL", "DELETE"],
  "synchronous_modes": ["NORMAL", "FULL"]
}
EOF

cc -O2 -o "${HELPER_BIN}" "${HELPER_SRC}" -lsqlite3

append_summary() {
  local raw_json="$1"
  local scenario="$2"
  node -e '
    const fs = require("fs");
    const [rawJson, scenario] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(rawJson, "utf8"));
    console.log(`scenario=${scenario}`);
    console.log(`iterations=${data.iterations}`);
    console.log(`payload_size=${data.payload_size}`);
    console.log(`journal_mode=${data.journal_mode}`);
    console.log(`synchronous=${data.synchronous}`);
    console.log(`commit_count=${data.commit_count}`);
    console.log(`sync_call_count=${data.sync_call_count}`);
    console.log(`total_seconds=${data.total_seconds}`);
    console.log(`avg_commit_latency_us=${data.avg_commit_latency_us}`);
    console.log(`p50_commit_latency_us=${data.p50_commit_latency_us}`);
    console.log(`p95_commit_latency_us=${data.p95_commit_latency_us}`);
    console.log(`p99_commit_latency_us=${data.p99_commit_latency_us}`);
    console.log(`raw_json=${rawJson}`);
    console.log("");
  ' "${raw_json}" "${scenario}" >> "${SUMMARY_FILE}"
}

for iterations in 1000 10000; do
  for journal in WAL DELETE; do
    for synchronous in NORMAL FULL; do
      scenario="$(printf '%s_%s_%s' "$(printf '%s' "${journal}" | tr '[:upper:]' '[:lower:]')" "$(printf '%s' "${synchronous}" | tr '[:upper:]' '[:lower:]')" "${iterations}")"
      db_path="${RESULT_DIR}/${scenario}.sqlite3"
      raw_json="${RAW_DIR}/${scenario}.json"
      rm -f "${db_path}" "${db_path}-wal" "${db_path}-shm"
      bench_log "sqlite commit latency journal=${journal} synchronous=${synchronous} iterations=${iterations}"
      "${HELPER_BIN}" "${db_path}" "${journal}" "${synchronous}" "${iterations}" "1024" "${raw_json}"
      append_summary "${raw_json}" "${scenario}"
    done
  done
done

bench_log "sqlite commit latency results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
