# VFS Validation Plan

## 目的

この repo の検証は次の順番で進める。

1. まず `VFS` を検証する
2. `VFS` が十分と判断できたら `llm-wiki` を検証する

この切り分けを優先する理由は、現状の `llm-wiki` がまず `FS-first substrate` として成立しているかを固める段階だからです。
wiki としての品質評価を先に混ぜると、保存・同期・検索・競合制御の問題と、知識ベース運用の問題が混線しやすくなります。

## VFS 検証方針

公開ベンチと repo 専用テストを分けて扱う。

### 1. 基礎 I/O

- ツール: `fio`
- 目的: 順次 read/write、ランダム read/write、`fsync` を含む遅延確認
- 観測値: throughput、IOPS、p50/p95/p99 latency と実行設定

### 2. Small File / Metadata

- ツール: `run_smallfile_vfs.sh`
- 目的: 小ファイル大量作成、list、stat、read、delete の癖を確認する
- 意味: `llm-wiki` の mirror workload に近い

### 3. SQLite 基礎

- ツール: `SQLite speedtest1`
- 目的: SQLite 自体の transaction / insert / update / search の基準を知る
- 意味: DB 側の上限感を把握する
- 追加: single-row transaction の commit latency を別ベンチで見る

### 4. Repo 専用 VFS Workload

実際に測る API は次を対象にする。

- `write_node`
- `append_node`
- `edit_node`
- `move_node`
- `delete_node`
- `list_nodes`
- `glob_nodes`
- `search_nodes`
- `export_snapshot`
- `fetch_updates`

## VFS で必ず測るシナリオ

### 正常系

- 1KB, 4KB, 16KB, 64KB の markdown を新規作成する
- 既存 node に小さな追記を行う
- 既存 node に plain-text edit を行う
- rename を行い、新 path が見え、旧 path が消えることを確認する
- tombstone 後に同 path revive ができることを確認する

### 競合制御

- `etag` 一致時の更新
- `etag` 不一致時の更新失敗
- delete 時の `etag` 不一致失敗

### 探索と検索

- 1,000 / 10,000 node 下での `list_nodes`
- 深い階層での `glob_nodes("**/*.md")`
- FTS ありでの `search_nodes`

### 同期

- `fetch_updates` の空差分
- `fetch_updates` の小差分
- rename 後に `removed_paths + changed_nodes` が期待通り返ること
- tombstone を含む場合の差分が崩れないこと

## VFS の合格条件

### 正しさ

- CRUD、move、search、sync 差分が壊れない
- `etag` 衝突が期待通り失敗する
- tombstone と revive が一貫する

### 性能

- node 数が増えても `list_nodes` / `search_nodes` / `fetch_updates` が急に崩れない
- 小変更時に full refresh に逃げず差分同期できる
- 単発更新で transaction コストが許容範囲に収まる

### 運用性

- mirror で conflict が観測できる
- Obsidian 側で pull / push / delete が安定する

## VFS の次に行う llm-wiki 検証

VFS が終わったら、ここからは「知識ベースとして成立しているか」を見る。

- `index.md` を入口に辿れるか
- source から page 更新までの運用が回るか
- citation が本文近くに残るか
- orphan page を検出できるか
- search が navigation の補助として十分か
- 人間編集と agent 編集が破綻しないか

## おすすめの実施順

1. 既存の Rust テストを土台に VFS correctness を拡張する
2. `criterion` か簡単な専用 benchmark コマンドで repo 専用 VFS workload を追加する
3. 必要なら `fio` / `smallfile` で外部比較を行う
4. VFS 合格後に `llm-wiki` の運用シナリオ試験へ進む

## 実行手順

VFS 段階の最小実行セットは次の通り。

```bash
cargo test --workspace
cd plugins/kinic-wiki && npm run check
bash scripts/build-wiki-canister-canbench.sh
```

`canbench` 実行環境がある場合は追加で次を使う。

```bash
bash scripts/run_canbench_guard.sh
```

`bash scripts/bench/run_all_vfs_benchmarks.sh` は guard も試すが、runtime 不在や PocketIC version 不一致では外部ベンチを優先して skip する。

チェック項目と未カバー一覧は `VFS_CORRECTNESS_CHECKLIST.md` を参照する。
外部ベンチの実行手順は `VFS_EXTERNAL_BENCHMARKS.md` を参照する。
