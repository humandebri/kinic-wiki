// Where: crates/wiki_cli/src/bin/vfs_bench/common.rs
// What: Shared benchmark args, path helpers, and latency aggregation for deployed canister benches.
// Why: The workload and latency runners should share one source of truth for scenario labels and metrics.
use clap::ValueEnum;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryShape {
    Flat,
    Fanout100x100,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Temperature {
    ColdSeeded,
    WarmRepeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadOperation {
    Create,
    RenameSameDir,
    RenameCrossDir,
    Delete,
    ReadSingle,
    ListPrefix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum LatencyOperation {
    WriteNode,
    AppendNode,
}

#[derive(Clone, Debug, Serialize)]
pub struct LatencyStats {
    pub request_count: usize,
    pub total_seconds: f64,
    pub avg_latency_us: f64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

pub fn make_payload(payload_size_bytes: usize) -> String {
    "x".repeat(payload_size_bytes)
}

pub fn file_path(prefix: &str, shape: DirectoryShape, index: usize) -> String {
    match shape {
        DirectoryShape::Flat => format!("{prefix}/node-{index:06}.md"),
        DirectoryShape::Fanout100x100 => format!(
            "{prefix}/l1-{l1:02}/l2-{l2:02}/node-{leaf:02}-{index:06}.md",
            l1 = (index / 10_000) % 100,
            l2 = (index / 100) % 100,
            leaf = index % 100
        ),
    }
}

pub fn same_dir_renamed_path(prefix: &str, shape: DirectoryShape, index: usize) -> String {
    match shape {
        DirectoryShape::Flat => format!("{prefix}/node-{index:06}.renamed.md"),
        DirectoryShape::Fanout100x100 => format!(
            "{prefix}/l1-{l1:02}/l2-{l2:02}/node-{leaf:02}-{index:06}.renamed.md",
            l1 = (index / 10_000) % 100,
            l2 = (index / 100) % 100,
            leaf = index % 100
        ),
    }
}

pub fn cross_dir_renamed_path(prefix: &str, shape: DirectoryShape, index: usize) -> String {
    match shape {
        DirectoryShape::Flat => format!("{prefix}/moved/node-{index:06}.md"),
        DirectoryShape::Fanout100x100 => format!(
            "{prefix}/xmove/l1-{l1:02}/l2-{l2:02}/node-{leaf:02}-{index:06}.md",
            l1 = ((index / 10_000) + 1) % 100,
            l2 = ((index / 100) + 1) % 100,
            leaf = index % 100
        ),
    }
}

pub fn list_prefix(prefix: &str, shape: DirectoryShape) -> String {
    match shape {
        DirectoryShape::Flat => prefix.to_string(),
        DirectoryShape::Fanout100x100 => format!("{prefix}/l1-00/l2-00"),
    }
}

pub fn latency_stats(latencies_us: &[u64], total_seconds: f64) -> LatencyStats {
    let mut sorted = latencies_us.to_vec();
    sorted.sort_unstable();
    let count = sorted.len();
    let total_us = sorted.iter().copied().sum::<u64>();
    LatencyStats {
        request_count: count,
        total_seconds,
        avg_latency_us: if count == 0 {
            0.0
        } else {
            total_us as f64 / count as f64
        },
        p50_latency_us: percentile(&sorted, 50),
        p95_latency_us: percentile(&sorted, 95),
        p99_latency_us: percentile(&sorted, 99),
    }
}

fn percentile(sorted: &[u64], pct: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((pct as f64 / 100.0) * (sorted.len().saturating_sub(1)) as f64).floor() as usize;
    sorted[index]
}

pub fn shard_bounds(total: usize, shard_count: usize, shard_index: usize) -> (usize, usize) {
    let start = (total * shard_index) / shard_count;
    let end = (total * (shard_index + 1)) / shard_count;
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::{
        DirectoryShape, cross_dir_renamed_path, file_path, latency_stats, list_prefix,
        same_dir_renamed_path,
    };

    #[test]
    fn flat_paths_are_stable() {
        assert_eq!(
            file_path("/Wiki/bench", DirectoryShape::Flat, 12),
            "/Wiki/bench/node-000012.md"
        );
        assert_eq!(
            same_dir_renamed_path("/Wiki/bench", DirectoryShape::Flat, 12),
            "/Wiki/bench/node-000012.renamed.md"
        );
        assert_eq!(
            cross_dir_renamed_path("/Wiki/bench", DirectoryShape::Flat, 12),
            "/Wiki/bench/moved/node-000012.md"
        );
        assert_eq!(
            list_prefix("/Wiki/bench", DirectoryShape::Flat),
            "/Wiki/bench"
        );
    }

    #[test]
    fn fanout_paths_are_stable() {
        assert_eq!(
            file_path("/Wiki/bench", DirectoryShape::Fanout100x100, 12_345),
            "/Wiki/bench/l1-01/l2-23/node-45-012345.md"
        );
        assert_eq!(
            same_dir_renamed_path("/Wiki/bench", DirectoryShape::Fanout100x100, 12_345),
            "/Wiki/bench/l1-01/l2-23/node-45-012345.renamed.md"
        );
        assert_eq!(
            list_prefix("/Wiki/bench", DirectoryShape::Fanout100x100),
            "/Wiki/bench/l1-00/l2-00"
        );
    }

    #[test]
    fn latency_stats_use_sorted_percentiles() {
        let stats = latency_stats(&[10, 40, 20, 30, 100], 0.1);
        assert_eq!(stats.request_count, 5);
        assert_eq!(stats.p50_latency_us, 30);
        assert_eq!(stats.p95_latency_us, 40);
        assert_eq!(stats.p99_latency_us, 40);
    }
}
