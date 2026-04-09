// Where: crates/wiki_cli/src/bin/vfs_bench/workload.rs
// What: Run deployed-canister workload scenarios that mirror smallfile-style operations.
// Why: We want FS-shaped inputs against a real canister without relying on canbench.
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use tokio::task::JoinSet;
use wiki_cli::client::{CanisterWikiClient, WikiApi};
use wiki_types::{
    DeleteNodeRequest, ListNodesRequest, MoveNodeRequest, NodeKind, WriteNodeRequest,
};

use crate::vfs_bench::common::{
    DirectoryShape, Temperature, WorkloadOperation, cross_dir_renamed_path, file_path,
    latency_stats, list_prefix, make_payload, same_dir_renamed_path, shard_bounds,
};

#[derive(Clone, Debug)]
pub struct WorkloadBenchArgs {
    pub benchmark_name: String,
    pub replica_host: String,
    pub canister_id: String,
    pub prefix: String,
    pub payload_size_bytes: usize,
    pub file_count: usize,
    pub directory_shape: DirectoryShape,
    pub concurrent_clients: usize,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub temperature: Temperature,
    pub operation: WorkloadOperation,
}

#[derive(Debug, Serialize)]
pub struct WorkloadBenchResult {
    pub benchmark_name: String,
    pub replica_host: String,
    pub canister_id: String,
    pub prefix: String,
    pub operation: WorkloadOperation,
    pub temperature: Temperature,
    pub directory_shape: DirectoryShape,
    pub payload_size_bytes: usize,
    pub file_count: usize,
    pub concurrent_clients: usize,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub request_count: usize,
    pub seed_seconds: f64,
    pub wall_seconds: f64,
    pub total_seconds: f64,
    pub ops_per_sec: f64,
    pub avg_latency_us: f64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

pub async fn run_workload_bench(args: WorkloadBenchArgs) -> Result<WorkloadBenchResult> {
    let client = Arc::new(CanisterWikiClient::new(&args.replica_host, &args.canister_id).await?);
    run_workload_bench_with_client(client, args).await
}

async fn run_workload_bench_with_client<C>(
    client: Arc<C>,
    args: WorkloadBenchArgs,
) -> Result<WorkloadBenchResult>
where
    C: WikiApi + Send + Sync + 'static,
{
    let payload = make_payload(args.payload_size_bytes);
    let measure_prefix = format!("{}/measure", args.prefix);

    if args.temperature == Temperature::WarmRepeat && args.warmup_iterations > 0 {
        warm_prefix(&client, &args, &measure_prefix, &payload).await?;
    }

    let wall_started_at = Instant::now();
    let execution = execute_operation(
        &client,
        &args,
        &measure_prefix,
        &payload,
        should_reuse_seeded_prefix(&args),
    )
    .await?;
    let wall_seconds = wall_started_at.elapsed().as_secs_f64();
    let stats = latency_stats(&execution.latencies, execution.measured_seconds);
    Ok(WorkloadBenchResult {
        benchmark_name: args.benchmark_name,
        replica_host: args.replica_host,
        canister_id: args.canister_id,
        prefix: measure_prefix,
        operation: args.operation,
        temperature: args.temperature,
        directory_shape: args.directory_shape,
        payload_size_bytes: args.payload_size_bytes,
        file_count: args.file_count,
        concurrent_clients: args.concurrent_clients,
        iterations: args.iterations,
        warmup_iterations: args.warmup_iterations,
        request_count: stats.request_count,
        seed_seconds: execution.seed_seconds,
        wall_seconds,
        total_seconds: stats.total_seconds,
        ops_per_sec: if stats.total_seconds == 0.0 {
            0.0
        } else {
            stats.request_count as f64 / stats.total_seconds
        },
        avg_latency_us: stats.avg_latency_us,
        p50_latency_us: stats.p50_latency_us,
        p95_latency_us: stats.p95_latency_us,
        p99_latency_us: stats.p99_latency_us,
    })
}

struct OperationExecution {
    latencies: Vec<u64>,
    seed_seconds: f64,
    measured_seconds: f64,
}

async fn warm_prefix<C>(
    client: &Arc<C>,
    args: &WorkloadBenchArgs,
    prefix: &str,
    payload: &str,
) -> Result<()>
where
    C: WikiApi + Send + Sync + 'static,
{
    match args.operation {
        WorkloadOperation::Create => {
            for _ in 0..args.warmup_iterations {
                clear_operation_paths(client, prefix, args, false).await?;
                let _ = create_nodes(client, prefix, args, payload).await?;
                clear_operation_paths(client, prefix, args, false).await?;
            }
        }
        WorkloadOperation::RenameSameDir => {
            for _ in 0..args.warmup_iterations {
                clear_operation_paths(client, prefix, args, true).await?;
                let etags = seed_nodes(client, prefix, args, payload).await?;
                let _ = rename_nodes(client, prefix, args, false, etags).await?;
                clear_operation_paths(client, prefix, args, true).await?;
            }
        }
        WorkloadOperation::RenameCrossDir => {
            for _ in 0..args.warmup_iterations {
                clear_operation_paths(client, prefix, args, true).await?;
                let etags = seed_nodes(client, prefix, args, payload).await?;
                let _ = rename_nodes(client, prefix, args, true, etags).await?;
                clear_operation_paths(client, prefix, args, true).await?;
            }
        }
        WorkloadOperation::Delete => {
            for _ in 0..args.warmup_iterations {
                clear_operation_paths(client, prefix, args, false).await?;
                let etags = seed_nodes(client, prefix, args, payload).await?;
                let _ = delete_nodes(client, prefix, args, etags).await?;
                clear_operation_paths(client, prefix, args, false).await?;
            }
        }
        WorkloadOperation::ReadSingle => {
            clear_operation_paths(client, prefix, args, false).await?;
            let _ = seed_nodes(client, prefix, args, payload).await?;
            for _ in 0..args.warmup_iterations {
                let _ = read_nodes(client, prefix, args).await?;
            }
        }
        WorkloadOperation::ListPrefix => {
            clear_operation_paths(client, prefix, args, false).await?;
            let _ = seed_nodes(client, prefix, args, payload).await?;
            for _ in 0..args.warmup_iterations {
                let _ = list_nodes(client, prefix, args).await?;
            }
        }
    }
    Ok(())
}

fn should_reuse_seeded_prefix(args: &WorkloadBenchArgs) -> bool {
    args.temperature == Temperature::WarmRepeat
        && args.warmup_iterations > 0
        && matches!(
            args.operation,
            WorkloadOperation::ReadSingle | WorkloadOperation::ListPrefix
        )
}

async fn execute_operation<C>(
    client: &Arc<C>,
    args: &WorkloadBenchArgs,
    prefix: &str,
    payload: &str,
    reuse_seeded_prefix: bool,
) -> Result<OperationExecution>
where
    C: WikiApi + Send + Sync + 'static,
{
    match args.operation {
        WorkloadOperation::Create => {
            let measured_started_at = Instant::now();
            let latencies = create_nodes(client, prefix, args, payload).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds: 0.0,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
        WorkloadOperation::RenameSameDir => {
            let seed_started_at = Instant::now();
            let etags = seed_nodes(client, prefix, args, payload).await?;
            let seed_seconds = seed_started_at.elapsed().as_secs_f64();
            let measured_started_at = Instant::now();
            let latencies = rename_nodes(client, prefix, args, false, etags).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
        WorkloadOperation::RenameCrossDir => {
            let seed_started_at = Instant::now();
            let etags = seed_nodes(client, prefix, args, payload).await?;
            let seed_seconds = seed_started_at.elapsed().as_secs_f64();
            let measured_started_at = Instant::now();
            let latencies = rename_nodes(client, prefix, args, true, etags).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
        WorkloadOperation::Delete => {
            let seed_started_at = Instant::now();
            let etags = seed_nodes(client, prefix, args, payload).await?;
            let seed_seconds = seed_started_at.elapsed().as_secs_f64();
            let measured_started_at = Instant::now();
            let latencies = delete_nodes(client, prefix, args, etags).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
        WorkloadOperation::ReadSingle => {
            let seed_seconds = if reuse_seeded_prefix {
                0.0
            } else {
                let seed_started_at = Instant::now();
                seed_nodes(client, prefix, args, payload).await?;
                seed_started_at.elapsed().as_secs_f64()
            };
            let measured_started_at = Instant::now();
            let latencies = read_nodes(client, prefix, args).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
        WorkloadOperation::ListPrefix => {
            let seed_seconds = if reuse_seeded_prefix {
                0.0
            } else {
                let seed_started_at = Instant::now();
                seed_nodes(client, prefix, args, payload).await?;
                seed_started_at.elapsed().as_secs_f64()
            };
            let measured_started_at = Instant::now();
            let latencies = list_nodes(client, prefix, args).await?;
            Ok(OperationExecution {
                latencies,
                seed_seconds,
                measured_seconds: measured_started_at.elapsed().as_secs_f64(),
            })
        }
    }
}

async fn clear_operation_paths<C>(
    client: &Arc<C>,
    prefix: &str,
    args: &WorkloadBenchArgs,
    include_move_targets: bool,
) -> Result<()>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    let operation = args.operation;
    let file_count = args.file_count;
    let concurrent_clients = args.concurrent_clients;
    let total = if include_move_targets {
        file_count * 2
    } else {
        file_count
    };
    run_parallel_actions(concurrent_clients, total, move |index| {
        let client = Arc::clone(&client);
        let path = if include_move_targets && index >= file_count {
            let target_index = index - file_count;
            match operation {
                WorkloadOperation::RenameCrossDir => {
                    cross_dir_renamed_path(&prefix, shape, target_index)
                }
                WorkloadOperation::RenameSameDir => {
                    same_dir_renamed_path(&prefix, shape, target_index)
                }
                _ => file_path(&prefix, shape, target_index),
            }
        } else {
            file_path(&prefix, shape, index)
        };
        async move {
            if let Some(node) = client.read_node(&path).await? {
                client
                    .delete_node(DeleteNodeRequest {
                        path,
                        expected_etag: Some(node.etag),
                    })
                    .await?;
            }
            Ok::<(), anyhow::Error>(())
        }
    })
    .await
}

async fn seed_nodes<C>(
    client: &Arc<C>,
    prefix: &str,
    args: &WorkloadBenchArgs,
    payload: &str,
) -> Result<Vec<String>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    let payload = payload.to_string();
    let etags = Arc::new(Mutex::new(vec![None; args.file_count]));
    let shared_etags = Arc::clone(&etags);
    run_parallel_actions(args.concurrent_clients, args.file_count, move |index| {
        let client = Arc::clone(&client);
        let path = file_path(&prefix, shape, index);
        let content = payload.clone();
        let etags = Arc::clone(&shared_etags);
        async move {
            let result = client
                .write_node(WriteNodeRequest {
                    path,
                    kind: NodeKind::File,
                    content,
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                })
                .await?;
            etags.lock().expect("etag mutex poisoned")[index] = Some(result.node.etag);
            Ok::<(), anyhow::Error>(())
        }
    })
    .await?;
    let etags = etags.lock().expect("etag mutex poisoned");
    Ok(etags
        .iter()
        .map(|etag| etag.clone().expect("seed should record etag"))
        .collect())
}

async fn create_nodes<C>(
    client: &Arc<C>,
    prefix: &str,
    args: &WorkloadBenchArgs,
    payload: &str,
) -> Result<Vec<u64>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    let payload = payload.to_string();
    run_parallel(args.concurrent_clients, args.file_count, move |index| {
        let client = Arc::clone(&client);
        let path = file_path(&prefix, shape, index);
        let content = payload.clone();
        async move {
            timed_update(async move {
                client
                    .write_node(WriteNodeRequest {
                        path,
                        kind: NodeKind::File,
                        content,
                        metadata_json: "{}".to_string(),
                        expected_etag: None,
                    })
                    .await
                    .map(|_| ())
            })
            .await
        }
    })
    .await
}

async fn rename_nodes<C>(
    client: &Arc<C>,
    prefix: &str,
    args: &WorkloadBenchArgs,
    cross_dir: bool,
    etags: Vec<String>,
) -> Result<Vec<u64>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    let etags = Arc::new(etags);
    run_parallel(args.concurrent_clients, args.file_count, move |index| {
        let client = Arc::clone(&client);
        let from_path = file_path(&prefix, shape, index);
        let to_path = if cross_dir {
            cross_dir_renamed_path(&prefix, shape, index)
        } else {
            same_dir_renamed_path(&prefix, shape, index)
        };
        let etag = etags[index].clone();
        async move {
            timed_update(async move {
                client
                    .move_node(MoveNodeRequest {
                        from_path,
                        to_path,
                        expected_etag: Some(etag),
                        overwrite: false,
                    })
                    .await
                    .map(|_| ())
            })
            .await
        }
    })
    .await
}

async fn delete_nodes<C>(
    client: &Arc<C>,
    prefix: &str,
    args: &WorkloadBenchArgs,
    etags: Vec<String>,
) -> Result<Vec<u64>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    let etags = Arc::new(etags);
    run_parallel(args.concurrent_clients, args.file_count, move |index| {
        let client = Arc::clone(&client);
        let path = file_path(&prefix, shape, index);
        let etag = etags[index].clone();
        async move {
            timed_update(async move {
                client
                    .delete_node(DeleteNodeRequest {
                        path,
                        expected_etag: Some(etag),
                    })
                    .await
                    .map(|_| ())
            })
            .await
        }
    })
    .await
}

async fn read_nodes<C>(client: &Arc<C>, prefix: &str, args: &WorkloadBenchArgs) -> Result<Vec<u64>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let prefix = prefix.to_string();
    let shape = args.directory_shape;
    run_parallel(args.concurrent_clients, args.file_count, move |index| {
        let client = Arc::clone(&client);
        let path = file_path(&prefix, shape, index);
        async move { timed_update(async move { client.read_node(&path).await.map(|_| ()) }).await }
    })
    .await
}

async fn list_nodes<C>(client: &Arc<C>, prefix: &str, args: &WorkloadBenchArgs) -> Result<Vec<u64>>
where
    C: WikiApi + Send + Sync + 'static,
{
    let client = Arc::clone(client);
    let list_prefix = list_prefix(prefix, args.directory_shape);
    run_parallel(args.concurrent_clients, args.iterations, move |_| {
        let client = Arc::clone(&client);
        let list_prefix = list_prefix.clone();
        async move {
            timed_update(async move {
                client
                    .list_nodes(ListNodesRequest {
                        prefix: list_prefix,
                        recursive: false,
                        include_deleted: false,
                    })
                    .await
                    .map(|_| ())
            })
            .await
        }
    })
    .await
}

async fn run_parallel<F, Fut>(concurrent_clients: usize, total: usize, build: F) -> Result<Vec<u64>>
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<u64>> + Send + 'static,
{
    let build = Arc::new(build);
    let mut join_set: JoinSet<Result<Vec<u64>>> = JoinSet::new();
    for shard_index in 0..concurrent_clients {
        let (start, end) = shard_bounds(total, concurrent_clients, shard_index);
        let build = Arc::clone(&build);
        join_set.spawn(async move {
            let mut latencies = Vec::with_capacity(end.saturating_sub(start));
            for index in start..end {
                latencies.push(build(index).await?);
            }
            Ok::<Vec<u64>, anyhow::Error>(latencies)
        });
    }

    let mut latencies = Vec::with_capacity(total);
    while let Some(result) = join_set.join_next().await {
        latencies.extend(result??);
    }
    Ok(latencies)
}

async fn run_parallel_actions<F, Fut>(
    concurrent_clients: usize,
    total: usize,
    build: F,
) -> Result<()>
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
{
    let build = Arc::new(build);
    let mut join_set: JoinSet<Result<()>> = JoinSet::new();
    for shard_index in 0..concurrent_clients {
        let (start, end) = shard_bounds(total, concurrent_clients, shard_index);
        let build = Arc::clone(&build);
        join_set.spawn(async move {
            for index in start..end {
                build(index).await?;
            }
            Ok::<(), anyhow::Error>(())
        });
    }

    while let Some(result) = join_set.join_next().await {
        result??;
    }
    Ok(())
}

async fn timed_update<F>(future: F) -> Result<u64>
where
    F: std::future::Future<Output = Result<()>>,
{
    let started_at = Instant::now();
    future.await?;
    Ok(started_at.elapsed().as_micros() as u64)
}

#[cfg(test)]
mod tests {
    use super::{WorkloadBenchArgs, run_workload_bench_with_client};
    use crate::vfs_bench::common::{DirectoryShape, Temperature, WorkloadOperation};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use wiki_cli::client::WikiApi;
    use wiki_types::{
        AppendNodeRequest, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest, EditNodeResult,
        ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest, FetchUpdatesResponse,
        GlobNodeHit, GlobNodesRequest, ListNodesRequest, MkdirNodeRequest, MkdirNodeResult,
        MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest, MultiEditNodeResult, Node,
        NodeEntry, RecentNodeHit, RecentNodesRequest, SearchNodeHit, SearchNodesRequest, Status,
        WriteNodeRequest, WriteNodeResult,
    };

    #[derive(Default)]
    struct MockBenchClient {
        next_etag: Mutex<u64>,
        nodes: Mutex<HashMap<String, Node>>,
        operations: Mutex<Vec<String>>,
    }

    impl MockBenchClient {
        fn record(&self, operation: String) {
            self.operations
                .lock()
                .expect("operations should lock")
                .push(operation);
        }

        fn next_etag(&self) -> String {
            let mut next = self.next_etag.lock().expect("etag counter should lock");
            *next += 1;
            format!("etag-{}", *next)
        }
    }

    #[async_trait]
    impl WikiApi for MockBenchClient {
        async fn status(&self) -> Result<Status> {
            Ok(Status {
                file_count: 0,
                source_count: 0,
                deleted_count: 0,
            })
        }

        async fn read_node(&self, path: &str) -> Result<Option<Node>> {
            self.record(format!("read:{path}"));
            Ok(self
                .nodes
                .lock()
                .expect("nodes should lock")
                .get(path)
                .cloned())
        }

        async fn list_nodes(&self, request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
            self.record(format!("list:{}", request.prefix));
            Ok(Vec::new())
        }

        async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
            self.record(format!("write:{}", request.path));
            let node = Node {
                path: request.path.clone(),
                kind: request.kind,
                content: request.content,
                created_at: 1,
                updated_at: 2,
                etag: self.next_etag(),
                deleted_at: None,
                metadata_json: request.metadata_json,
            };
            self.nodes
                .lock()
                .expect("nodes should lock")
                .insert(request.path, node.clone());
            Ok(WriteNodeResult {
                created: true,
                node,
            })
        }

        async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
            unreachable!()
        }

        async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
            unreachable!()
        }

        async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
            self.record(format!("delete:{}", request.path));
            let removed = self
                .nodes
                .lock()
                .expect("nodes should lock")
                .remove(&request.path)
                .expect("delete target should exist");
            Ok(DeleteNodeResult {
                path: request.path,
                etag: removed.etag,
                deleted_at: 3,
            })
        }

        async fn move_node(&self, request: MoveNodeRequest) -> Result<MoveNodeResult> {
            self.record(format!("move:{}->{}", request.from_path, request.to_path));
            let mut nodes = self.nodes.lock().expect("nodes should lock");
            let mut moved = nodes
                .remove(&request.from_path)
                .expect("move source should exist");
            moved.path = request.to_path.clone();
            moved.etag = self.next_etag();
            nodes.insert(request.to_path.clone(), moved.clone());
            Ok(MoveNodeResult {
                from_path: request.from_path,
                node: moved,
                overwrote: false,
            })
        }

        async fn mkdir_node(&self, _request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
            unreachable!()
        }

        async fn glob_nodes(&self, _request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
            Ok(Vec::new())
        }

        async fn recent_nodes(&self, _request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
            Ok(Vec::new())
        }

        async fn multi_edit_node(
            &self,
            _request: MultiEditNodeRequest,
        ) -> Result<MultiEditNodeResult> {
            unreachable!()
        }

        async fn search_nodes(&self, _request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
            Ok(Vec::new())
        }

        async fn export_snapshot(
            &self,
            _request: ExportSnapshotRequest,
        ) -> Result<ExportSnapshotResponse> {
            Ok(ExportSnapshotResponse {
                snapshot_revision: "snap".to_string(),
                nodes: Vec::new(),
            })
        }

        async fn fetch_updates(
            &self,
            _request: FetchUpdatesRequest,
        ) -> Result<FetchUpdatesResponse> {
            Ok(FetchUpdatesResponse {
                snapshot_revision: "snap".to_string(),
                changed_nodes: Vec::new(),
                removed_paths: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn warm_repeat_reads_reuse_the_measured_prefix() {
        let client = Arc::new(MockBenchClient::default());
        let result = run_workload_bench_with_client(
            Arc::clone(&client),
            WorkloadBenchArgs {
                benchmark_name: "warm-read".to_string(),
                replica_host: "http://127.0.0.1:4943".to_string(),
                canister_id: "aaaaa-aa".to_string(),
                prefix: "/Wiki/bench".to_string(),
                payload_size_bytes: 32,
                file_count: 2,
                directory_shape: DirectoryShape::Flat,
                concurrent_clients: 1,
                iterations: 2,
                warmup_iterations: 2,
                temperature: Temperature::WarmRepeat,
                operation: WorkloadOperation::ReadSingle,
            },
        )
        .await
        .expect("warm repeat should succeed");

        assert_eq!(result.prefix, "/Wiki/bench/measure");
        assert_eq!(result.seed_seconds, 0.0);

        let operations = client
            .operations
            .lock()
            .expect("operations should lock")
            .clone();
        let write_paths = operations
            .iter()
            .filter(|entry| entry.starts_with("write:"))
            .cloned()
            .collect::<Vec<_>>();
        let read_paths = operations
            .iter()
            .filter(|entry| entry.starts_with("read:"))
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(write_paths.len(), 2);
        assert!(
            write_paths
                .iter()
                .all(|entry| entry.starts_with("write:/Wiki/bench/measure/"))
        );
        assert!(read_paths.len() >= 6);
        assert!(
            read_paths
                .iter()
                .all(|entry| entry.starts_with("read:/Wiki/bench/measure/"))
        );
        assert!(!operations.iter().any(|entry| entry.contains("/warmup-")));
    }
}
