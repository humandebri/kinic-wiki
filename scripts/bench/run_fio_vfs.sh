#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/run_fio_vfs.sh
# What: Run tiered fio scenarios and persist raw/config/environment/summary artifacts.
# Why: Public VFS review needs a small comparable core plus extended fixed-cost diagnostics.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

require_command "fio" "Install fio first. Example: brew install fio or apt-get install fio."
require_command "node" "Node.js is required to summarize fio JSON output."

RESULT_DIR="$(bench_results_dir "fio")"
RAW_DIR="$(bench_raw_dir "${RESULT_DIR}")"
WORK_DIR="$(bench_work_dir "fio")"
SUMMARY_FILE="${RESULT_DIR}/summary.txt"
CONFIG_FILE="${RESULT_DIR}/config.json"
ENVIRONMENT_FILE="${RESULT_DIR}/environment.json"
write_summary_header "${SUMMARY_FILE}" "fio"
write_environment_json "${ENVIRONMENT_FILE}"

FILE_SIZE="256m"
RUNTIME_SECONDS=30
IOENGINE="sync"
IO_DEPTH=1
NUMJOBS=1
THREAD=1
DIRECT=0
FDATASYNC=0
CACHE_DROP="false"
if [[ "$(uname -s)" == "Darwin" ]]; then
  SYNC_FILE_RANGE="unsupported"
else
  SYNC_FILE_RANGE="0"
fi

SCENARIO_DEFS="$(cat <<'EOF'
sequential_read_64k|core|read|64k|0|cold,warm
sequential_write_64k|core|write|64k|0|cold,warm
random_read_4k|core|randread|4k|0|cold,warm
random_write_4k|core|randwrite|4k|0|cold,warm
fsync_per_write_4k|core|write|4k|1|cold,warm
sequential_read_4k|extended|read|4k|0|warm
sequential_write_4k|extended|write|4k|0|warm
random_read_64k|extended|randread|64k|0|warm
random_write_64k|extended|randwrite|64k|0|warm
EOF
)"

node -e '
  const fs = require("fs");
  const [configFile, defs, syncFileRange] = process.argv.slice(1);
  const scenarios = defs.trim().split("\n").filter(Boolean).flatMap(line => {
    const [name, tier, rw, bs, fsync, temperatures] = line.split("|");
    return temperatures.split(",").map(temperature => ({
      name,
      tier,
      bs,
      rw,
      direct: 0,
      iodepth: 1,
      numjobs: 1,
      thread: 1,
      ioengine: "sync",
      runtime: 30,
      size: "256m",
      fsync: Number(fsync),
      fdatasync: 0,
      sync_file_range: syncFileRange,
      cache_drop: false,
      temperature,
      path_strategy: temperature === "cold" ? "fresh_path" : "reused_path_after_warmup"
    }));
  });
  const payload = {
    tool: "fio",
    defaults: {
      direct: 0,
      iodepth: 1,
      numjobs: 1,
      thread: 1,
      ioengine: "sync",
      runtime: 30,
      size: "256m",
      fdatasync: 0,
      sync_file_range: syncFileRange,
      cache_drop: false
    },
    scenarios
  };
  fs.writeFileSync(configFile, JSON.stringify(payload, null, 2) + "\n");
' "${CONFIG_FILE}" "${SCENARIO_DEFS}" "${SYNC_FILE_RANGE}"

run_fio() {
  local output_json="$1"
  shift
  fio "$@" --output-format=json --output="${output_json}"
}

prepare_seed_file() {
  local target_file="$1"
  local output_json="$2"
  run_fio "${output_json}" \
    --name=seed_file \
    --filename="${target_file}" \
    --size="${FILE_SIZE}" \
    --rw=write \
    --bs=64k \
    --ioengine="${IOENGINE}" \
    --iodepth="${IO_DEPTH}" \
    --numjobs="${NUMJOBS}" \
    --thread="${THREAD}" \
    --direct="${DIRECT}" \
    --time_based=0 \
    --fdatasync="${FDATASYNC}"
}

summarize_fio() {
  local output_json="$1"
  local scenario_name="$2"
  local tier="$3"
  local rw_mode="$4"
  local block_size="$5"
  local fsync_value="$6"
  local temperature="$7"
  local path_strategy="$8"
  node -e '
    const fs = require("fs");
    const [
      outputJson,
      scenarioName,
      tier,
      rwMode,
      blockSize,
      fsyncValue,
      temperature,
      pathStrategy
    ] = process.argv.slice(1);
    const data = JSON.parse(fs.readFileSync(outputJson, "utf8"));
    const job = data.jobs[0];
    const section = rwMode.includes("read") ? job.read : job.write;
    const latency = section.clat_ns ?? {};
    const percentiles = latency.percentile ?? {};
    const field = key => percentiles[key] ?? percentiles[`${key}0`] ?? "NA";
    const latencySection = rwMode.includes("read") ? "read" : "write";
    const latencyMeaning = Number(fsyncValue) === 1
      ? "completion latency for write jobs with fsync=1"
      : "completion latency";
    const lines = [
      `scenario=${scenarioName}`,
      `tier=${tier}`,
      `block_size=${blockSize}`,
      `rw=${rwMode}`,
      `temperature=${temperature}`,
      `path_strategy=${pathStrategy}`,
      "direct=0",
      "iodepth=1",
      "numjobs=1",
      "thread=1",
      "ioengine=sync",
      "runtime=30",
      "size=256m",
      `fsync=${fsyncValue}`,
      "fdatasync=0",
      `sync_file_range=${process.platform === "darwin" ? "unsupported" : "0"}`,
      "cache_drop=false",
      `latency_source=jobs[0].${latencySection}.clat_ns | ${latencyMeaning}`,
      `throughput_bytes_per_sec=${section.bw_bytes}`,
      `iops=${section.iops}`,
      `avg_latency_ns=${latency.mean ?? "NA"}`,
      `p50_latency_ns=${field("50.000000")}`,
      `p95_latency_ns=${field("95.000000")}`,
      `p99_latency_ns=${field("99.000000")}`,
      `raw_json=${outputJson}`,
      ""
    ];
    console.log(lines.join("\n"));
  ' "${output_json}" "${scenario_name}" "${tier}" "${rw_mode}" "${block_size}" "${fsync_value}" "${temperature}" "${path_strategy}" >> "${SUMMARY_FILE}"
}

run_case() {
  local scenario_name="$1"
  local tier="$2"
  local rw_mode="$3"
  local block_size="$4"
  local fsync_value="$5"
  local temperature="$6"
  local target_file="${WORK_DIR}/${scenario_name}.dat"
  local output_json="${RAW_DIR}/${scenario_name}_${temperature}.json"
  local path_strategy

  if [[ "${temperature}" == "cold" ]]; then
    path_strategy="fresh_path"
  else
    path_strategy="reused_path_after_warmup"
  fi

  if [[ "${rw_mode}" == *read* ]]; then
    prepare_seed_file "${target_file}" "${RAW_DIR}/${scenario_name}_${temperature}_seed.json"
  elif [[ "${temperature}" == "cold" ]]; then
    rm -f "${target_file}"
  elif [[ ! -f "${target_file}" ]]; then
    prepare_seed_file "${target_file}" "${RAW_DIR}/${scenario_name}_${temperature}_seed.json"
  fi

  if [[ "${temperature}" == "warm" ]]; then
    run_fio "${RAW_DIR}/${scenario_name}_${temperature}_warmup.json" \
      --name="${scenario_name}_warmup" \
      --filename="${target_file}" \
      --size="${FILE_SIZE}" \
      --rw="${rw_mode}" \
      --bs="${block_size}" \
      --ioengine="${IOENGINE}" \
      --iodepth="${IO_DEPTH}" \
      --numjobs="${NUMJOBS}" \
      --thread="${THREAD}" \
      --direct="${DIRECT}" \
      --time_based=1 \
      --runtime="${RUNTIME_SECONDS}" \
      --fdatasync="${FDATASYNC}" \
      --fsync="${fsync_value}" >/dev/null 2>&1
  fi

  bench_log "fio ${scenario_name} tier=${tier} temperature=${temperature}"
  run_fio "${output_json}" \
    --name="${scenario_name}_${temperature}" \
    --filename="${target_file}" \
    --size="${FILE_SIZE}" \
    --rw="${rw_mode}" \
    --bs="${block_size}" \
    --ioengine="${IOENGINE}" \
    --iodepth="${IO_DEPTH}" \
    --numjobs="${NUMJOBS}" \
    --thread="${THREAD}" \
    --direct="${DIRECT}" \
    --time_based=1 \
    --runtime="${RUNTIME_SECONDS}" \
    --fdatasync="${FDATASYNC}" \
    --fsync="${fsync_value}"

  summarize_fio \
    "${output_json}" \
    "${scenario_name}" \
    "${tier}" \
    "${rw_mode}" \
    "${block_size}" \
    "${fsync_value}" \
    "${temperature}" \
    "${path_strategy}"
}

while IFS='|' read -r scenario_name tier rw_mode block_size fsync_value temperatures; do
  [[ -z "${scenario_name}" ]] && continue
  IFS=',' read -r -a temperature_list <<< "${temperatures}"
  for temperature in "${temperature_list[@]}"; do
    run_case "${scenario_name}" "${tier}" "${rw_mode}" "${block_size}" "${fsync_value}" "${temperature}"
  done
done <<< "${SCENARIO_DEFS}"

bench_log "fio results saved to ${RESULT_DIR}"
printf 'summary=%s\n' "${SUMMARY_FILE}"
