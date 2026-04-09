// Where: crates/wiki_cli/src/bin/vfs_bench/latency.rs
// What: Run single-update latency scenarios against a deployed canister.
// Why: We need a canister-side analogue to durable mutation latency without using canbench.
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use wiki_cli::client::{CanisterWikiClient, WikiApi};
use wiki_types::{AppendNodeRequest, NodeKind, WriteNodeRequest};

use crate::vfs_bench::common::{LatencyOperation, latency_stats, make_payload};

#[derive(Clone, Debug)]
pub struct LatencyBenchArgs {
    pub benchmark_name: String,
    pub replica_host: String,
    pub canister_id: String,
    pub prefix: String,
    pub payload_size_bytes: usize,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub operation: LatencyOperation,
}

#[derive(Debug, Serialize)]
pub struct LatencyBenchResult {
    pub benchmark_name: String,
    pub replica_host: String,
    pub canister_id: String,
    pub prefix: String,
    pub operation: LatencyOperation,
    pub payload_size_bytes: usize,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub request_count: usize,
    pub total_seconds: f64,
    pub avg_latency_us: f64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

pub async fn run_latency_bench(args: LatencyBenchArgs) -> Result<LatencyBenchResult> {
    let client = Arc::new(CanisterWikiClient::new(&args.replica_host, &args.canister_id).await?);
    let path = format!("{}/measure/node.md", args.prefix);
    let payload = make_payload(args.payload_size_bytes);

    let mut current_etag = seed_base_node(&client, &path, &payload).await?;
    for _ in 0..args.warmup_iterations {
        let (_, next_etag) =
            run_latency_request(&client, &path, &payload, args.operation, &current_etag).await?;
        current_etag = next_etag;
    }

    let started_at = Instant::now();
    let mut latencies = Vec::with_capacity(args.iterations);
    for _ in 0..args.iterations {
        let (latency_us, next_etag) =
            run_latency_request(&client, &path, &payload, args.operation, &current_etag).await?;
        latencies.push(latency_us);
        current_etag = next_etag;
    }
    let total_seconds = started_at.elapsed().as_secs_f64();
    let stats = latency_stats(&latencies, total_seconds);

    Ok(LatencyBenchResult {
        benchmark_name: args.benchmark_name,
        replica_host: args.replica_host,
        canister_id: args.canister_id,
        prefix: args.prefix,
        operation: args.operation,
        payload_size_bytes: args.payload_size_bytes,
        iterations: args.iterations,
        warmup_iterations: args.warmup_iterations,
        request_count: stats.request_count,
        total_seconds: stats.total_seconds,
        avg_latency_us: stats.avg_latency_us,
        p50_latency_us: stats.p50_latency_us,
        p95_latency_us: stats.p95_latency_us,
        p99_latency_us: stats.p99_latency_us,
    })
}

async fn seed_base_node(
    client: &Arc<CanisterWikiClient>,
    path: &str,
    payload: &str,
) -> Result<String> {
    let result = client
        .write_node(WriteNodeRequest {
            path: path.to_string(),
            kind: NodeKind::File,
            content: payload.to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await?;
    Ok(result.node.etag)
}

async fn run_latency_request(
    client: &Arc<CanisterWikiClient>,
    path: &str,
    payload: &str,
    operation: LatencyOperation,
    current_etag: &str,
) -> Result<(u64, String)> {
    let started_at = Instant::now();
    let next_etag = match operation {
        LatencyOperation::WriteNode => {
            client
                .write_node(WriteNodeRequest {
                    path: path.to_string(),
                    kind: NodeKind::File,
                    content: payload.to_string(),
                    metadata_json: "{}".to_string(),
                    expected_etag: Some(current_etag.to_string()),
                })
                .await?
                .node
                .etag
        }
        LatencyOperation::AppendNode => {
            client
                .append_node(AppendNodeRequest {
                    path: path.to_string(),
                    content: payload.to_string(),
                    expected_etag: Some(current_etag.to_string()),
                    separator: None,
                    metadata_json: None,
                    kind: None,
                })
                .await?
                .node
                .etag
        }
    };
    Ok((started_at.elapsed().as_micros() as u64, next_etag))
}
