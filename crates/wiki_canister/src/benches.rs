// Where: crates/wiki_canister/src/benches.rs
// What: canbench benchmarks for the FS-first canister entrypoints.
// Why: CI should detect instruction and memory regressions on the operations this branch changes most heavily.
use std::hint::black_box;

use canbench_rs::{BenchResult, bench, bench_fn};
use wiki_types::{
    AppendNodeRequest, EditNodeRequest, ExportSnapshotRequest, FetchUpdatesRequest,
    MoveNodeRequest, MultiEdit, MultiEditNodeRequest, NodeKind, RecentNodesRequest,
    SearchNodesRequest, WriteNodeRequest,
};

use crate::{SERVICE, initialize_service, read_node, write_node};

fn ensure_bench_service() {
    let initialized = SERVICE.with(|slot| slot.borrow().is_some());
    if !initialized {
        initialize_service().expect("bench service should initialize");
    }
}

fn seed_file(path: &str, content: &str) -> String {
    ensure_bench_service();
    let expected_etag = read_node(path.to_string())
        .expect("seed read should succeed")
        .map(|node| node.etag);
    write_node(WriteNodeRequest {
        path: path.to_string(),
        kind: NodeKind::File,
        content: content.to_string(),
        metadata_json: "{}".to_string(),
        expected_etag,
    })
    .expect("seed write should succeed")
    .node
    .etag
}

#[bench(raw)]
fn write_node_bench() -> BenchResult {
    let path = "/Wiki/bench/write.md";
    let expected_etag = Some(seed_file(path, "seed content"));
    bench_fn(|| {
        black_box(
            write_node(WriteNodeRequest {
                path: path.to_string(),
                kind: NodeKind::File,
                content: "updated content".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: expected_etag.clone(),
            })
            .expect("bench write should succeed"),
        );
    })
}

#[bench(raw)]
fn append_node_bench() -> BenchResult {
    let path = "/Wiki/bench/append.md";
    let expected_etag = Some(seed_file(path, "alpha"));
    bench_fn(|| {
        black_box(
            crate::append_node(AppendNodeRequest {
                path: path.to_string(),
                content: "beta".to_string(),
                expected_etag: expected_etag.clone(),
                separator: Some("\n".to_string()),
                metadata_json: None,
                kind: None,
            })
            .expect("bench append should succeed"),
        );
    })
}

#[bench(raw)]
fn edit_node_bench() -> BenchResult {
    let path = "/Wiki/bench/edit.md";
    let expected_etag = Some(seed_file(path, "alpha beta"));
    bench_fn(|| {
        black_box(
            crate::edit_node(EditNodeRequest {
                path: path.to_string(),
                old_text: "beta".to_string(),
                new_text: "gamma".to_string(),
                expected_etag: expected_etag.clone(),
                replace_all: false,
            })
            .expect("bench edit should succeed"),
        );
    })
}

#[bench(raw)]
fn move_node_bench() -> BenchResult {
    let from_path = "/Wiki/bench/move/from.md";
    let to_path = "/Wiki/bench/move/to.md";
    let expected_etag = Some(seed_file(from_path, "move me"));
    bench_fn(|| {
        black_box(
            crate::move_node(MoveNodeRequest {
                from_path: from_path.to_string(),
                to_path: to_path.to_string(),
                expected_etag: expected_etag.clone(),
                overwrite: true,
            })
            .expect("bench move should succeed"),
        );
    })
}

#[bench(raw)]
fn search_nodes_bench() -> BenchResult {
    seed_file("/Wiki/bench/search/one.md", "alpha benchmark target");
    seed_file("/Wiki/bench/search/two.md", "beta benchmark helper");
    bench_fn(|| {
        black_box(
            crate::search_nodes(SearchNodesRequest {
                query_text: "benchmark".to_string(),
                prefix: Some("/Wiki/bench/search".to_string()),
                top_k: 10,
            })
            .expect("bench search should succeed"),
        );
    })
}

#[bench(raw)]
fn fetch_updates_bench() -> BenchResult {
    let path = "/Wiki/bench/fetch/note.md";
    seed_file(path, "alpha");
    let snapshot = crate::export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki/bench/fetch".to_string()),
        include_deleted: false,
    })
    .expect("snapshot export should succeed");
    black_box(seed_file(path, "alpha beta"));
    bench_fn(|| {
        black_box(
            crate::fetch_updates(FetchUpdatesRequest {
                known_snapshot_revision: snapshot.snapshot_revision.clone(),
                prefix: Some("/Wiki/bench/fetch".to_string()),
                include_deleted: false,
            })
            .expect("bench fetch_updates should succeed"),
        );
    })
}

#[bench(raw)]
fn multi_edit_node_bench() -> BenchResult {
    let path = "/Wiki/bench/multi-edit.md";
    let expected_etag = Some(seed_file(path, "alpha beta gamma"));
    bench_fn(|| {
        black_box(
            crate::multi_edit_node(MultiEditNodeRequest {
                path: path.to_string(),
                edits: vec![
                    MultiEdit {
                        old_text: "alpha".to_string(),
                        new_text: "one".to_string(),
                    },
                    MultiEdit {
                        old_text: "gamma".to_string(),
                        new_text: "three".to_string(),
                    },
                ],
                expected_etag: expected_etag.clone(),
            })
            .expect("bench multi_edit should succeed"),
        );
    })
}

#[bench(raw)]
fn recent_nodes_bench() -> BenchResult {
    seed_file("/Wiki/bench/recent/a.md", "alpha");
    seed_file("/Wiki/bench/recent/b.md", "beta");
    bench_fn(|| {
        black_box(
            crate::recent_nodes(RecentNodesRequest {
                limit: 10,
                path: Some("/Wiki/bench/recent".to_string()),
                include_deleted: false,
            })
            .expect("bench recent should succeed"),
        );
    })
}
