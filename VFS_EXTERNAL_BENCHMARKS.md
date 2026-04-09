# VFS External Benchmarks

## 目的

この文書は、repo 内の契約テストと `canbench` だけでは見えにくい

- ローカル filesystem の素の I/O 特性
- small file / metadata workload
- SQLite 単独の基礎特性

を、公開されている一般的なベンチで観測するための手順書です。

この段階では `llm-wiki` 固有の wiki 品質は扱いません。
対象は VFS 検証フェーズに限定します。

## 採用する外部ベンチ

### `fio`

- 役割: sequential / random / fsync-heavy I/O の基礎観測
- 見る値: throughput, IOPS, p50/p95/p99 latency
- 実行入口: `bash scripts/bench/run_fio_vfs.sh`
- 公開 tier
  - `public_core`: `sequential_read_64k`, `sequential_write_64k`, `random_read_4k`, `random_write_4k`, `fsync_per_write_4k`
  - `public_extended`: `sequential_read_4k`, `sequential_write_4k`, `random_read_64k`, `random_write_64k`
- 補足: `core` は比較用、`extended` は 4k/64k の固定費診断用
- 補足: 各 scenario は `tier`, `direct`, `iodepth`, `numjobs`, `ioengine`, `runtime`, `size`, `fsync`, `fdatasync`, `sync_file_range`, `temperature`, `cache_drop`, `path_strategy` を summary と config に残す

### `smallfile`

- 役割: small file と metadata 寄りの workload を repo 内の固定スクリプトで観測する
- 見る値: create / unlink / rename_same_dir / rename_cross_dir / stat / readdir / small_append / open_close / mkdir_rmdir の秒数と ops/sec
- 実行入口: `bash scripts/bench/run_smallfile_vfs.sh`
- 補足: `process-based concurrency` を正式仕様とし、shared-memory worker は使わない
- 補足: `temperature = warm / cold_process_restart`、`sync_policy = none / per_op`、`directory_shape = flat_10000 / fanout_100x100` を config と summary に残す

### `SQLite speedtest1`

- 役割: SQLite smoke / compatibility / broad workload reference
- 見る値: total time
- 比較軸: `journal_mode = WAL / DELETE`, `synchronous = NORMAL / FULL`
- 実行入口: `bash scripts/bench/run_sqlite_speedtest1.sh`

### `SQLite commit latency`

- 役割: single-row transaction の durable commit コストを切り出す主評価
- 見る値: total_seconds, avg/p50/p95/p99 commit latency
- 実行入口: `bash scripts/bench/run_sqlite_commit_latency_vfs.sh`

## 前提ツール

必要ツール:

- `fio`
- `sqlite3`
- `cc`
- `node`

補足:

- `run_sqlite_speedtest1.sh` は SQLite 公式の `speedtest1.c` を自動取得しません
- `speedtest1.c` は次のいずれかに手動配置します
  - `./.benchmarks/sqlite/vendor/speedtest1.c`
  - `./.benchmarks/sqlite/speedtest1.c`
  - `SQLITE_SPEEDTEST1_SOURCE=/abs/path/to/speedtest1.c`
- 参照元: [SQLite speedtest1.c](https://sqlite.org/src/doc/tip/test/speedtest1.c)

## 実行手順

個別実行:

```bash
bash scripts/bench/run_fio_vfs.sh
bash scripts/bench/run_smallfile_vfs.sh
bash scripts/bench/run_sqlite_speedtest1.sh
bash scripts/bench/run_sqlite_commit_latency_vfs.sh
```

まとめて実行:

```bash
bash scripts/bench/run_all_vfs_benchmarks.sh
```

補足:

- `run_all_vfs_benchmarks.sh` は `canbench` guard も試す
- ただし PocketIC runtime が見つからない場合、または version が `pocket-ic-server 10.0.0` と一致しない場合は skip して外部ベンチに進む

## デプロイ済み canister bench

この節は host FS bench の補助です。
`fio`、`smallfile`、SQLite のようなローカル filesystem ベンチとは混ぜません。
対象は、すでにデプロイ済みの canister を `ic-agent` 経由で外から叩く bench です。

### 役割

| Bench | 役割 | 実行入口 |
| --- | --- | --- |
| `canister_vfs_workload` | `smallfile` に近い入力形を `write_node` / `move_node` / `delete_node` / `read_node` / `list_nodes` で測る | `REPLICA_HOST=... CANISTER_ID=... bash scripts/bench/run_canister_vfs_workload.sh` |
| `canister_vfs_latency` | `write_node` / `append_node` の `1 request = 1 mutation` latency を測る | `REPLICA_HOST=... CANISTER_ID=... bash scripts/bench/run_canister_vfs_latency.sh` |

### workload bench の固定入力

- operation: `create`, `rename_same_dir`, `rename_cross_dir`, `delete`, `read_single`, `list_prefix`
- directory shape: `flat`, `fanout_100x100`
- temperature: `cold_seeded`, `warm_repeat`
- file count: `10^2`, `10^3`, `10^4`, `10^5`
- payload size: `1k`, `4k`
- concurrent clients: `1`, `4`, `8`

補足:

- `rename_same_dir` は同じ parent prefix 内で rename
- `rename_cross_dir` は別 parent prefix へ rename
- `list_prefix` は `flat` では 1 prefix 全体、`fanout_100x100` では 1 branch prefix を対象にする

### latency bench の固定入力

- scenario: `write_node_single_1k`, `write_node_single_4k`, `append_node_single_1k`, `append_node_single_4k`
- iterations: `1000`
- warmup iterations: `20`
- raw は全件保存ではなく histogram 集計型 JSON

### 出力先

```text
.benchmarks/results/canister_vfs_workload/<timestamp>/
.benchmarks/results/canister_vfs_latency/<timestamp>/
```

各 run には次が入ります。

- `summary.txt`
- `config.json`
- `environment.json`
- `raw/*.json`

`environment.json` には通常の bench 情報に加えて、最低限次を残します。

- `replica_host`
- `canister_id`
- `bench_transport = ic-agent`

## 出力先

出力はすべて repo 配下の無視パスに保存します。

```text
.benchmarks/results/<tool>/<timestamp>/
```

各 run には少なくとも次が入ります。

- `config.json`
- `environment.json`
- raw 結果
- `summary.txt`

## 各ベンチの意味

### `fio`

これは VFS API ではなく、下のファイルシステムとストレージの基礎を見るものです。
`write_node` や `append_node` が遅いときに、まず生 I/O が遅いのかを切り分けるために使います。
`core` は比較用、`extended` は 4k/64k 差の診断用です。
`temperature` と `fsync=1` を分けて保存するので、小さい durable write と固定費支配の両方を見やすくなります。

### `smallfile`

これは Obsidian mirror に近い small file / metadata workload を repo 内で固定して観測するベンチです。
rename は same-dir と cross-dir を分けます。
さらに open/close、mkdir/rmdir、sync-per-op 系も分けるので、どの metadata 操作と durability 操作が支配的かを切り分けやすいです。

### `SQLite speedtest1`

これは SQLite 単独の基準です。
公開上の位置づけは「参考値」です。
`search_nodes` や `fetch_updates` の遅さが VFS ロジックではなく SQLite 側に寄っているかを切り分けます。

### `SQLite commit latency`

これは `1 transaction = 1 row` のコストを見るベンチです。
公開上の位置づけは「durability-sensitive workload の主評価」です。
`llm-wiki` の小さい durable update に近く、`speedtest1` より commit の重さを直接見られます。

### `canbench`

これは canister API ごとの instruction / memory regression を見るもので、ローカル FS ベンチとは役割が違います。
`fio` や `smallfile` と直接比較しません。
`snapshot size`、`export_snapshot`、`fetch_updates` の scale は canbench 側の責務であり、この文書では扱いません。

### `deployed canister bench`

これは host FS の素性能ではなく、デプロイ済み canister に対する API workload を見る bench です。
外部 filesystem-style bench と同じ表には混ぜず、VFS API を外から叩いた時の request/latency を観測します。
`smallfile` に近い入力形を持ちますが、block I/O や OS metadata そのものを測るわけではありません。

## 結果の読み方

- `fio`
  - throughput と IOPS が低い: 生 I/O に制約がある可能性
  - fsync-heavy write だけ遅い: durable write 系 workload のコストが高い可能性
- `smallfile`
  - create/delete が遅い: mirror workload で small file 操作が支配的な可能性
  - rename cross-dir / mkdir-rmdir / open-close が遅い: metadata 固定費が支配的な可能性
  - list/stat が遅い: directory traversal や metadata 操作が支配的な可能性
- `SQLite speedtest1`
  - `WAL/NORMAL` と `DELETE/FULL` の差が大きい: durability 設定が性能に強く効いている可能性
- `SQLite commit latency`
  - p95/p99 が高い: 単発更新の durable commit が tail latency を支配している可能性

## このベンチで分かること / 分からないこと

分かること:

- ローカル FS の sequential / random / fsync-heavy 傾向
- small file / metadata workload の傾向
- SQLite の journal / synchronous 差

分からないこと:

- `llm-wiki` のナビゲーション品質
- citation や orphan page の品質
- agent と人間の wiki 運用品質

それらは VFS 検証後の `llm-wiki` 段階で別に検証します。

## 現在の取得結果

この repo で現時点に取得できている外部ベンチ結果は次です。

| Bench | Timestamp | Status | Notes |
| --- | --- | --- | --- |
| `smallfile` | `20260409T040921Z` | collected | metadata v1 一式を取得 |
| `sqlite_commit_latency` | `20260409T040921Z` | collected | durability 主評価 |
| `sqlite_speedtest1` | `20260409T041037Z` | collected | broad reference workload |
| `fio` | - | not collected | `failed to setup shm segment` |

### 実行環境メモ

共通の `environment.json` ではおおむね次の環境が記録されています。

| Item | Value |
| --- | --- |
| OS | `Darwin 25.3.0` |
| CPU | `arm64` |
| PocketIC | `pocket-ic-server 10.0.0` |
| Rust | `rustc 1.93.0` |
| Node | `v22.22.0` |
| SQLite | `3.51.0` |
| fio | `fio-3.42` |

### `fio`

| Item | Value |
| --- | --- |
| Status | not collected |
| Error | `failed to setup shm segment` |
| Minimal repro | `--thread=1` と `--parse-only` でも同じ |
| Read | wrapper の scenario 構成ではなく `fio` 起動時の shared memory 初期化で落ちている |

### `SQLite commit latency`

主評価としては `WAL` が明確に軽く、`DELETE` が重いです。

`10000` iteration の代表値:

| Scenario | Avg | p95 | p99 |
| --- | --- | --- | --- |
| `wal_normal_10000` | `83.84us` | `87us` | `737us` |
| `wal_full_10000` | `194.488us` | `302us` | `1589us` |
| `delete_normal_10000` | `2555.719us` | `8840us` | `35786us` |
| `delete_full_10000` | `2865.333us` | `9204us` | `29407us` |

`1000` iteration 側でも同じ傾向で、`WAL` は `DELETE` より大幅に軽いです。
この結果は「小さい durable update の主コストは journal/synchronous 設定に強く依存する」という読みを支持します。

参照:

- [summary.txt](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_commit_latency/20260409T040921Z/summary.txt)
- [config.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_commit_latency/20260409T040921Z/config.json)
- [environment.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_commit_latency/20260409T040921Z/environment.json)

### `SQLite speedtest1`

参考値としても `WAL` が軽く、`DELETE` がやや重いです。

| Scenario | Total Time |
| --- | --- |
| `wal_normal` | `2.34s` |
| `wal_full` | `2.45s` |
| `delete_normal` | `2.81s` |
| `delete_full` | `2.96s` |

`commit latency` ほど差は鋭くないですが、durability 設定差の方向性は同じです。

参照:

- [summary.txt](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_speedtest1/20260409T041037Z/summary.txt)
- [config.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_speedtest1/20260409T041037Z/config.json)
- [environment.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/sqlite_speedtest1/20260409T041037Z/environment.json)

### `smallfile`

metadata 系では `rename_same_dir`、`rename_cross_dir`、`mkdir_rmdir` が相対的に重く、`stat` と `open_close` はかなり軽いです。
`flat_10000` と `fanout_100x100`、`warm` と `cold_process_restart`、`clients=1/4/8`、sync-per-op 4 本が取得できています。

`warm_flat_10000` の代表値:

| Operation | Total | Ops/Sec |
| --- | --- | --- |
| `create` | `4.195s` | `2383.68` |
| `unlink` | `2.234s` | `4475.85` |
| `rename_same_dir` | `11.171s` | `895.17` |
| `rename_cross_dir` | `6.466s` | `1546.47` |
| `stat` | `0.276s` | `36199.85` |
| `open_close` | `0.608s` | `16459.78` |
| `mkdir_rmdir` | `2.899s` | `3449.92` |

この結果から、small file workload では rename と directory mutation の固定費が目立ち、単純な metadata read は比較的軽いと読めます。

参照:

- [summary.txt](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/smallfile/20260409T040921Z/summary.txt)
- [config.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/smallfile/20260409T040921Z/config.json)
- [environment.json](/Users/0xhude/Desktop/work/llm-wiki/.benchmarks/results/smallfile/20260409T040921Z/environment.json)
