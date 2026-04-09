# VFS Deployed Canister Benchmarks

## 目的

この文書は、deploy 済み canister を `ic-agent` 経由で直接叩いた bench 結果をまとめるものです。
`fio` / `smallfile` / SQLite の host FS bench とは分けて読みます。

対象:

- deployed canister workload bench
- deployed canister single-update latency bench

## 実行先

| Item | Value |
| --- | --- |
| Replica Host | `http://127.0.0.1:8000` |
| Canister ID | `t63gs-up777-77776-aaaba-cai` |
| Transport | `ic-agent` |
| Network | `icp local` |

## 取得済み run

| Bench | Timestamp | Notes |
| --- | --- | --- |
| `canister_vfs_latency` | `20260409T052945Z` | `iterations=250`, `warmup=10` に絞って取得 |
| `canister_vfs_workload` | `20260409T053943Z` | `file_count=100`, `payload=1k`, `clients=1` の代表 subset |

## Latency Bench

条件:

| Item | Value |
| --- | --- |
| Iterations | `250` |
| Warmup Iterations | `10` |
| Scenarios | `write_node_single_{1k,4k}`, `append_node_single_{1k,4k}` |

結果:

| Scenario | Avg | p50 | p95 | p99 | Total Seconds |
| --- | --- | --- | --- | --- | --- |
| `write_node_single_1k` | `782404us` | `558818us` | `1137158us` | `6733688us` | `195.60s` |
| `write_node_single_4k` | `586994us` | `555637us` | `702565us` | `1543817us` | `146.75s` |
| `append_node_single_1k` | `427964us` | `410552us` | `500970us` | `677958us` | `106.99s` |
| `append_node_single_4k` | `389660us` | `364871us` | `466692us` | `696775us` | `97.42s` |

読み:

- `write_node` は `append_node` より明確に重い
- tail latency は `write_node_single_1k` が特に大きい
- local canister update の固定費がかなり強い

参照:

- [summary.txt](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_latency/20260409T052945Z/summary.txt)
- [config.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_latency/20260409T052945Z/config.json)
- [environment.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_latency/20260409T052945Z/environment.json)

## Workload Bench

計時定義:

| Field | Meaning |
| --- | --- |
| `seed_seconds` | 測定対象 operation の前に投入した seed phase の時間 |
| `total_seconds` | measured operation 自体の時間 |
| `wall_seconds` | seed を含む run 全体の壁時計時間 |

今回の取得条件:

| Item | Value |
| --- | --- |
| File Count | `100` |
| Payload Size | `1024B` |
| Concurrent Clients | `1` |
| Directory Shapes | `flat`, `fanout100x100` |
| Temperatures | `cold_seeded`, `warm_repeat` |
| Operations | `create`, `rename_same_dir`, `rename_cross_dir`, `delete`, `read_single`, `list_prefix` |

### Create

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `create_flat_cold_seeded_n100_p1024_c1` | `47.53s` | `2.10` | `475281us` | `730847us` | `1007088us` |
| `create_flat_warm_repeat_n100_p1024_c1` | `46.37s` | `2.16` | `463688us` | `931990us` | `1535327us` |
| `create_fanout100x100_cold_seeded_n100_p1024_c1` | `35.03s` | `2.85` | `350261us` | `489084us` | `604693us` |
| `create_fanout100x100_warm_repeat_n100_p1024_c1` | `24.60s` | `4.06` | `246017us` | `268591us` | `318365us` |

### Rename Same Dir

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `rename_same_dir_flat_cold_seeded_n100_p1024_c1` | `50.70s` | `1.97` | `251847us` | `274670us` | `321491us` |
| `rename_same_dir_flat_warm_repeat_n100_p1024_c1` | `50.43s` | `1.98` | `256556us` | `272330us` | `346372us` |
| `rename_same_dir_fanout100x100_cold_seeded_n100_p1024_c1` | `50.43s` | `1.98` | `251579us` | `276412us` | `312009us` |
| `rename_same_dir_fanout100x100_warm_repeat_n100_p1024_c1` | `51.68s` | `1.93` | `261678us` | `297969us` | `498047us` |

### Rename Cross Dir

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `rename_cross_dir_flat_cold_seeded_n100_p1024_c1` | `51.19s` | `1.95` | `258420us` | `284270us` | `322990us` |
| `rename_cross_dir_flat_warm_repeat_n100_p1024_c1` | `54.06s` | `1.85` | `257475us` | `282693us` | `317092us` |
| `rename_cross_dir_fanout100x100_cold_seeded_n100_p1024_c1` | `51.79s` | `1.93` | `260018us` | `291313us` | `299204us` |
| `rename_cross_dir_fanout100x100_warm_repeat_n100_p1024_c1` | `55.20s` | `1.81` | `268729us` | `289907us` | `758639us` |

### Delete

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `delete_flat_cold_seeded_n100_p1024_c1` | `50.59s` | `1.98` | `248597us` | `261947us` | `276146us` |
| `delete_flat_warm_repeat_n100_p1024_c1` | `50.64s` | `1.97` | `255562us` | `281056us` | `504778us` |
| `delete_fanout100x100_cold_seeded_n100_p1024_c1` | `54.17s` | `1.85` | `285260us` | `443687us` | `725633us` |
| `delete_fanout100x100_warm_repeat_n100_p1024_c1` | `49.39s` | `2.02` | `246727us` | `257095us` | `265602us` |

### Read Single

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `read_single_flat_cold_seeded_n100_p1024_c1` | `25.18s` | `3.97` | `5995us` | `12974us` | `19254us` |
| `read_single_flat_warm_repeat_n100_p1024_c1` | `27.06s` | `3.70` | `6491us` | `7305us` | `31819us` |
| `read_single_fanout100x100_cold_seeded_n100_p1024_c1` | `26.31s` | `3.80` | `6559us` | `15083us` | `37721us` |
| `read_single_fanout100x100_warm_repeat_n100_p1024_c1` | `25.87s` | `3.87` | `6200us` | `6535us` | `35403us` |

### List Prefix

| Scenario | Total Seconds | Ops/Sec | Avg | p95 | p99 |
| --- | --- | --- | --- | --- | --- |
| `list_prefix_flat_cold_seeded_n100_p1024_c1` | `26.06s` | `3.84` | `7548us` | `26892us` | `57549us` |
| `list_prefix_flat_warm_repeat_n100_p1024_c1` | `25.79s` | `3.88` | `4723us` | `4178us` | `51247us` |
| `list_prefix_fanout100x100_cold_seeded_n100_p1024_c1` | `27.43s` | `3.65` | `4444us` | `3456us` | `43835us` |
| `list_prefix_fanout100x100_warm_repeat_n100_p1024_c1` | `26.29s` | `3.80` | `3366us` | `3225us` | `26667us` |

読み:

- update 系の `create` / `rename` / `delete` はおおむね `1.8` から `4.1 ops/s`
- `read_single` と `list_prefix` は query 系なので、update 系よりかなり軽い
- `create` は `fanout100x100 + warm_repeat` が最も軽い
- `rename_same_dir` と `rename_cross_dir` は shape 差より update 固定費の影響が強い

参照:

- [summary.txt](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_workload/20260409T053943Z/summary.txt)
- [config.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_workload/20260409T053943Z/config.json)
- [environment.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/canister_vfs_workload/20260409T053943Z/environment.json)

## 注意

- `canister_vfs_latency` は runtime を見て `iterations=250` に絞っている
- `canister_vfs_workload` は full matrix ではなく代表 subset
- 既存の `20260409T053943Z` workload run は seed/measured 分離前の値
- local canister の update がかなり重いため、full matrix はそのままでは非現実的
