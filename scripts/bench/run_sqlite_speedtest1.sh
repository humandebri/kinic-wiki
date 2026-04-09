#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_sqlite_speedtest1.sh
# What: Run SQLite's official speedtest1 workload under selected journal and synchronous modes.
# Why: VFS validation needs a standard SQLite baseline in addition to raw filesystem benchmarks.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

require_command "sqlite3" "Install sqlite3 first. It is required to execute the generated speedtest1 workload."
require_command "cc" "A C compiler is required to build speedtest1."
require_command "/usr/bin/time" "The external benchmark wrapper requires /usr/bin/time."
require_command "node" "Node.js is required to materialize environment metadata and summaries."

RESULT_DIR="$(bench_results_dir "sqlite_speedtest1")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
WORK_DIR="$(bench_work_dir "sqlite")"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
write_summary_header "${SUMMARY_FILE}" "sqlite_speedtest1"
write_environment_json "${ENVIRONMENT_FILE}"

cat > "${CONFIG_FILE}" <<'EOF'
{
  "tool": "sqlite_speedtest1",
  "benchmark_role": "broad SQLite reference workload",
  "size": 20,
  "testset": "main",
  "journal_modes": ["WAL", "DELETE"],
  "synchronous_modes": ["NORMAL", "FULL"]
}
EOF

SOURCE_CANDIDATES=(
  "${SQLITE_SPEEDTEST1_SOURCE:-}"
  "${WORK_DIR}/vendor/speedtest1.c"
  "${WORK_DIR}/speedtest1.c"
)

resolve_source() {
  local candidate
  for candidate in "${SOURCE_CANDIDATES[@]}"; do
    if [[ -n "${candidate}" && -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  echo "missing speedtest1.c source file." >&2
  echo "Place the official SQLite test source at ./.benchmarks/sqlite/vendor/speedtest1.c or set SQLITE_SPEEDTEST1_SOURCE=/abs/path/to/speedtest1.c" >&2
  echo "Reference: https://sqlite.org/src/doc/tip/test/speedtest1.c" >&2
  exit 1
}

SOURCE_FILE="$(resolve_source)"
BIN_DIR="${WORK_DIR}/bin"
BIN_PATH="${BIN_DIR}/speedtest1"
mkdir -p "${BIN_DIR}"

bench_log "compiling speedtest1 from ${SOURCE_FILE}"
cc -O2 -o "${BIN_PATH}" "${SOURCE_FILE}" -lsqlite3

SCRIPT_PATH="${RESULT_DIR}/speedtest1_workload.sql"
"${BIN_PATH}" --size 20 --testset main --script "${SCRIPT_PATH}" "${RESULT_DIR}/generator.sqlite3" >/dev/null
assert_file_exists "${SCRIPT_PATH}" "speedtest1 script generation failed"

run_sqlite_scenario() {
  local journal_mode="$1"
  local synchronous_mode="$2"
  local journal_lower
  local synchronous_lower
  local scenario
  local db_path
  local runner_script
  local stdout_file
  local time_file
  local total_seconds
  local raw_json

  journal_lower="$(printf '%s' "${journal_mode}" | tr '[:upper:]' '[:lower:]')"
  synchronous_lower="$(printf '%s' "${synchronous_mode}" | tr '[:upper:]' '[:lower:]')"
  scenario="${journal_lower}_${synchronous_lower}"
  db_path="${RESULT_DIR}/${scenario}.sqlite3"
  runner_script="${RESULT_DIR}/${scenario}.sql"
  stdout_file="${RESULT_DIR}/${scenario}.stdout"
  time_file="${RESULT_DIR}/${scenario}.time"
  raw_json="${RAW_DIR}/${scenario}.json"

  rm -f "${db_path}" "${db_path}-wal" "${db_path}-shm"
  cat > "${runner_script}" <<EOF
PRAGMA journal_mode=${journal_mode};
PRAGMA synchronous=${synchronous_mode};
.read ${SCRIPT_PATH}
EOF

  bench_log "sqlite speedtest1 journal=${journal_mode} synchronous=${synchronous_mode}"
  /usr/bin/time -p sqlite3 "${db_path}" < "${runner_script}" >"${stdout_file}" 2>"${time_file}"
  total_seconds="$(extract_time_real_seconds "${time_file}")"
  cat > "${raw_json}" <<EOF
{
  "scenario": "${scenario}",
  "journal_mode": "${journal_mode}",
  "synchronous": "${synchronous_mode}",
  "total_seconds": ${total_seconds},
  "database": "${db_path}"
}
EOF

  node -e '
    const fs = require("fs");
    const [rawJson] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(rawJson, "utf8"));
    const lines = [
      `scenario=${data.scenario}`,
      `journal_mode=${data.journal_mode}`,
      `synchronous=${data.synchronous}`,
      `total_seconds=${data.total_seconds}`,
      `database=${data.database}`,
      `raw_json=${rawJson}`,
      ""
    ];
    console.log(lines.join("\n"));
  ' "${raw_json}" >> "${SUMMARY_FILE}"
}

run_sqlite_scenario "WAL" "NORMAL"
run_sqlite_scenario "WAL" "FULL"
run_sqlite_scenario "DELETE" "NORMAL"
run_sqlite_scenario "DELETE" "FULL"

bench_log "sqlite speedtest1 results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
