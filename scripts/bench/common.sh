#!/usr/bin/env bash
set -euo pipefail

# Where: scripts/bench/common.sh
# What: Shared helpers for external VFS benchmark wrappers.
# Why: All benchmark scripts should resolve paths, failures, and output layout the same way.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BENCH_ROOT="${REPO_ROOT}/.benchmarks"
RESULTS_ROOT="${BENCH_ROOT}/results"
RUN_TIMESTAMP="${RUN_TIMESTAMP_OVERRIDE:-$(date -u +%Y%m%dT%H%M%SZ)}"

bench_repo_root() {
  printf '%s\n' "${REPO_ROOT}"
}

bench_results_dir() {
  local tool="$1"
  local dir="${RESULTS_ROOT}/${tool}/${RUN_TIMESTAMP}"
  mkdir -p "${dir}"
  printf '%s\n' "${dir}"
}

bench_raw_dir() {
  local result_dir="$1"
  local dir="${result_dir}/raw"
  mkdir -p "${dir}"
  printf '%s\n' "${dir}"
}

bench_work_dir() {
  local tool="$1"
  local dir="${BENCH_ROOT}/${tool}"
  mkdir -p "${dir}"
  printf '%s\n' "${dir}"
}

require_command() {
  local command_name="$1"
  local install_hint="$2"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    echo "${install_hint}" >&2
    exit 1
  fi
}

bench_log() {
  printf '[bench] %s\n' "$*"
}

write_summary_header() {
  local file="$1"
  local tool="$2"
  {
    printf 'tool=%s\n' "${tool}"
    printf 'timestamp=%s\n' "${RUN_TIMESTAMP}"
    printf 'repo_root=%s\n' "${REPO_ROOT}"
  } > "${file}"
}

extract_time_real_seconds() {
  local file="$1"
  awk '/^real / { print $2; exit }' "${file}"
}

assert_file_exists() {
  local path="$1"
  local message="$2"
  if [[ ! -f "${path}" ]]; then
    echo "${message}: ${path}" >&2
    exit 1
  fi
}

resolve_pocketic_bin() {
  local -a candidates=(
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

write_environment_json() {
  local file="$1"
  local os_name
  local kernel_version
  local cpu_name=""
  local memory_bytes=""
  local pocketic_version=""
  local rust_version=""
  local node_version=""
  local sqlite_version=""
  local fio_version=""
  local pocketic_bin=""

  os_name="$(uname -s)"
  kernel_version="$(uname -r)"

  if [[ "${os_name}" == "Darwin" ]]; then
    cpu_name="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
    memory_bytes="$(sysctl -n hw.memsize 2>/dev/null || true)"
  else
    cpu_name="$(lscpu 2>/dev/null | awk -F: '/Model name/ { gsub(/^[ \t]+/, "", $2); print $2; exit }')"
    memory_bytes="$(awk '/MemTotal:/ { print $2 * 1024; exit }' /proc/meminfo 2>/dev/null || true)"
  fi

  if [[ -z "${cpu_name}" ]]; then
    cpu_name="$(uname -m)"
  fi

  if pocketic_bin="$(resolve_pocketic_bin 2>/dev/null)"; then
    pocketic_version="$("${pocketic_bin}" --version 2>/dev/null || true)"
  fi
  if command -v rustc >/dev/null 2>&1; then
    rust_version="$(rustc --version 2>/dev/null || true)"
  fi
  if command -v node >/dev/null 2>&1; then
    node_version="$(node --version 2>/dev/null || true)"
  fi
  if command -v sqlite3 >/dev/null 2>&1; then
    sqlite_version="$(sqlite3 --version 2>/dev/null | awk '{print $1}')"
  fi
  if command -v fio >/dev/null 2>&1; then
    fio_version="$(fio --version 2>/dev/null || true)"
  fi

  node -e '
    const fs = require("fs");
    const [
      outputFile,
      osName,
      kernelVersion,
      cpuName,
      memoryBytes,
      pocketicVersion,
      rustVersion,
      nodeVersion,
      sqliteVersion,
      fioVersion
    ] = process.argv.slice(1);
    const asNullableString = value => value === "" ? null : value;
    const parsedMemory = memoryBytes === "" ? null : Number(memoryBytes);
    const payload = {
      os: osName,
      kernel_version: kernelVersion,
      cpu: asNullableString(cpuName),
      memory_bytes: Number.isFinite(parsedMemory) ? parsedMemory : null,
      pocketic_version: asNullableString(pocketicVersion),
      rust_version: asNullableString(rustVersion),
      node_version: asNullableString(nodeVersion),
      sqlite_version: asNullableString(sqliteVersion),
      fio_version: asNullableString(fioVersion)
    };
    fs.writeFileSync(outputFile, JSON.stringify(payload, null, 2) + "\n");
  ' \
    "${file}" \
    "${os_name}" \
    "${kernel_version}" \
    "${cpu_name}" \
    "${memory_bytes}" \
    "${pocketic_version}" \
    "${rust_version}" \
    "${node_version}" \
    "${sqlite_version}" \
    "${fio_version}"
}

augment_environment_json() {
  local file="$1"
  local replica_host="$2"
  local canister_id="$3"
  local bench_transport="$4"

  node -e '
    const fs = require("fs");
    const [filePath, replicaHost, canisterId, benchTransport] = process.argv.slice(1);
    const payload = JSON.parse(fs.readFileSync(filePath, "utf8"));
    payload.replica_host = replicaHost;
    payload.canister_id = canisterId;
    payload.bench_transport = benchTransport;
    fs.writeFileSync(filePath, JSON.stringify(payload, null, 2) + "\n");
  ' "${file}" "${replica_host}" "${canister_id}" "${bench_transport}"
}
