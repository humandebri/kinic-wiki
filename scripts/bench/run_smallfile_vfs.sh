#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_smallfile_vfs.sh
# What: Run metadata-focused scenarios with temperature and sync-policy labels.
# Why: Public VFS review needs metadata baselines plus durability-sensitive diagnostics.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

require_command "node" "Node.js is required to run the small-file benchmark."

RESULT_DIR="$(bench_results_dir "smallfile")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
WORK_DIR="$(bench_work_dir "smallfile")"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
RUNNER="${SCRIPT_DIR}/smallfile_runner.mjs"
write_summary_header "${SUMMARY_FILE}" "smallfile"
write_environment_json "${ENVIRONMENT_FILE}"

BASELINE_OPS="create,unlink,rename_same_dir,rename_cross_dir,stat,readdir_single,readdir_recursive,small_append,open_close,mkdir_rmdir"

SCENARIO_DEFS="$(cat <<EOF
warm_flat_10000|warm|none|10000|4096|10000|flat_10000|1|${BASELINE_OPS}
cold_flat_10000|cold_process_restart|none|10000|4096|10000|flat_10000|1|${BASELINE_OPS}
warm_fanout_100x100|warm|none|10000|4096|100|fanout_100x100|1|${BASELINE_OPS}
cold_fanout_100x100|cold_process_restart|none|10000|4096|100|fanout_100x100|1|${BASELINE_OPS}
file_size_1k|warm|none|10000|1024|100|fanout_100x100|1|${BASELINE_OPS}
file_size_4k|warm|none|10000|4096|100|fanout_100x100|1|${BASELINE_OPS}
file_size_16k|warm|none|10000|16384|100|fanout_100x100|1|${BASELINE_OPS}
file_size_64k|warm|none|10000|65536|100|fanout_100x100|1|${BASELINE_OPS}
file_count_1000|warm|none|1000|4096|100|fanout_100x100|1|${BASELINE_OPS}
file_count_10000|warm|none|10000|4096|100|fanout_100x100|1|${BASELINE_OPS}
file_count_50000|warm|none|50000|4096|100|fanout_100x100|1|${BASELINE_OPS}
clients_1|warm|none|10000|4096|100|fanout_100x100|1|${BASELINE_OPS}
clients_4|warm|none|10000|4096|100|fanout_100x100|4|${BASELINE_OPS}
clients_8|warm|none|10000|4096|100|fanout_100x100|8|${BASELINE_OPS}
create_sync_each|warm|per_op|1000|4096|100|fanout_100x100|1|create_sync_each
unlink_sync_each|warm|per_op|1000|4096|100|fanout_100x100|1|unlink_sync_each
small_append_sync_each|warm|per_op|1000|4096|100|fanout_100x100|1|small_append_sync_each
rename_cross_dir_sync_each|warm|per_op|1000|4096|100|fanout_100x100|1|rename_cross_dir_sync_each
EOF
)"

node -e '
  const fs = require("fs");
  const [configFile, defs] = process.argv.slice(1);
  const scenarios = defs.trim().split("\n").filter(Boolean).map(line => {
    const [scenario, temperature, syncPolicy, fileCount, fileSizeBytes, dirWidth, directoryShape, concurrentClients, operations] = line.split("|");
    return {
      scenario,
      temperature,
      sync_policy: syncPolicy,
      file_count: Number(fileCount),
      file_size_bytes: Number(fileSizeBytes),
      dir_width: Number(dirWidth),
      directory_shape: directoryShape,
      concurrent_clients: Number(concurrentClients),
      operations: operations.split(",")
    };
  });
  const payload = {
    tool: "smallfile",
    baseline: {
      file_size_bytes: 4096,
      file_count: 10000,
      dir_width: 100,
      directory_shape: "fanout_100x100",
      concurrent_clients: 1,
      temperature: "warm",
      sync_policy: "none"
    },
    sweeps: {
      file_size_bytes: [1024, 4096, 16384, 65536],
      file_count: [1000, 10000, 50000],
      concurrent_clients: [1, 4, 8],
      directory_shape: ["flat_10000", "fanout_100x100"],
      temperature: ["warm", "cold_process_restart"]
    },
    scenarios
  };
  fs.writeFileSync(configFile, JSON.stringify(payload, null, 2) + "\n");
' "${CONFIG_FILE}" "${SCENARIO_DEFS}"

append_summary() {
  local raw_json="$1"
  node -e '
    const fs = require("fs");
    const [rawJson] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(rawJson, "utf8"));
    for (const op of data.operations) {
      console.log(`scenario=${data.scenario}`);
      console.log(`temperature=${data.temperature}`);
      console.log(`sync_policy=${data.sync_policy}`);
      console.log(`file_count=${data.file_count}`);
      console.log(`file_size_bytes=${data.file_size_bytes}`);
      console.log(`dir_width=${data.dir_width}`);
      console.log(`directory_shape=${data.directory_shape}`);
      console.log(`concurrent_clients=${data.concurrent_clients}`);
      console.log(`operation=${op.operation}`);
      console.log(`ops_unit=${op.ops_unit}`);
      console.log(`total_seconds=${op.total_seconds}`);
      console.log(`ops_per_sec=${op.ops_per_sec}`);
      console.log(`raw_json=${rawJson}`);
      console.log("");
    }
  ' "${raw_json}" >> "${SUMMARY_FILE}"
}

run_case() {
  local scenario="$1"
  local temperature="$2"
  local sync_policy="$3"
  local file_count="$4"
  local file_size="$5"
  local dir_width="$6"
  local directory_shape="$7"
  local clients="$8"
  local operations="$9"
  local run_dir="${WORK_DIR}/${scenario}"
  local raw_json="${RAW_DIR}/${scenario}.json"
  bench_log "smallfile ${scenario}"
  node "${RUNNER}" parent \
    --scenario "${scenario}" \
    --run-dir "${run_dir}" \
    --raw-json "${raw_json}" \
    --temperature "${temperature}" \
    --sync-policy "${sync_policy}" \
    --file-count "${file_count}" \
    --file-size "${file_size}" \
    --dir-width "${dir_width}" \
    --directory-shape "${directory_shape}" \
    --clients "${clients}" \
    --operations "${operations}"
  append_summary "${raw_json}"
}

while IFS='|' read -r scenario temperature sync_policy file_count file_size dir_width directory_shape clients operations; do
  [[ -z "${scenario}" ]] && continue
  run_case \
    "${scenario}" \
    "${temperature}" \
    "${sync_policy}" \
    "${file_count}" \
    "${file_size}" \
    "${dir_width}" \
    "${directory_shape}" \
    "${clients}" \
    "${operations}"
done <<< "${SCENARIO_DEFS}"

bench_log "smallfile results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
