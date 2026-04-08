# FS First Plan

## 目的

canister の正本モデルを wiki 固有の `page/revision/section/system page` から切り離し、agent が自然に使える filesystem-first なモデルへ移行する。

この移行では、FS を UI や adapter ではなく第一級の公開モデルとする。
保存は relational DB を使い続けるが、公開 API と内部モデルの中心は `node(path, content, kind, ...)` に置く。

## 前提

- 互換 shim は入れない。
- 旧 wiki schema の自動吸収はしない。
- 破壊的変更として明示的に切り替える。
- `index.md` / `log.md` のような system page の自動生成はやめる。
- 競合解決は section 単位ではなく file 単位にする。
- 検索は current content に対してのみ行い、履歴検索は初期版では扱わない。

## 目標状態

### 保存モデル

中核は単一の `nodes` テーブルとする。

- `path TEXT PRIMARY KEY`
- `content TEXT NOT NULL`
- `kind TEXT NOT NULL`
- `created_at INTEGER NOT NULL`
- `updated_at INTEGER NOT NULL`
- `etag TEXT NOT NULL`
- `deleted_at INTEGER NULL`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`

永続化される `kind` は最小限に絞る。

- `file`
- `source`

`directory` は row として持たず、`path` prefix から仮想的に導出する。
list API の返り値でのみ仮想 directory entry を返す。

### 公開 API

canister API は wiki API ではなく FS API に寄せる。

- `read_node(path) -> opt Node`
- `list_nodes(prefix, recursive, include_deleted) -> vec NodeEntry`
- `write_node(path, content, kind, expected_etag) -> WriteResult`
- `delete_node(path, expected_etag) -> DeleteResult`
- `search_nodes(query, prefix, top_k) -> vec SearchHit`
- `export_snapshot(prefix) -> Snapshot`
- `fetch_updates(cursor or known_etags) -> Delta`

競合制御は `expected_etag` に一本化する。
現行の `expected_current_revision_id` は廃止する。

### 検索

検索は `nodes` の current content に対する FTS とする。

- FTS テーブル: `nodes_fts`
- index 対象: `deleted_at IS NULL` の node
- write/delete と同じ transaction で更新

## 捨てるもの

以下は新モデルでは第一級概念として持たない。

- `wiki_pages`
- `wiki_revisions`
- `wiki_sections`
- `system_pages`
- `log_events`
- section 単位 diff
- revision 単位 history API
- system page 自動再生成

必要ならそれらは agent が普通の file として表現する。

## 置き換え方針

### API の置き換え

- `get_page` は `read_node` に統合する
- `get_system_page` は廃止する
- `commit_wiki_changes` は複数 `write/delete` の batch API に置き換える
- `create_source` は `write_node` に統合するか、`kind=source` の thin wrapper にする
- `search` は `search_nodes` に置き換える

### mirror / sync の置き換え

Obsidian 側の `Wiki/` は remote nodes の working copy として扱う。

- pull: remote の `nodes` を path ベースで mirror する
- push: local file を path 単位で `write_node` / `delete_node` に反映する
- conflict: `etag` mismatch のみ扱う

frontmatter で page 固有 metadata を持つ前提はやめる。
必要なら mirror 管理用に最小限の hidden metadata を別ファイルで持つ。

## 段階的移行

### Phase 1: 型と API の確定

やること:

- `wiki_types` に FS-first の型を追加する
- `wiki.did` の新しい interface を定義する
- path ルール、etag ルール、delete semantics を固定する

決めること:

- path は `/Wiki/...` 形式にするか
- tombstone を返すか
- batch write/delete を初期版で入れるか

完了条件:

- Rust 型
- Candid interface
- API 契約メモ

が揃っていること。

### Phase 2: 新 store 実装

やること:

- `nodes` schema migration を追加する
- `nodes` の read/write/list/delete/search を実装する
- `nodes_fts` を実装する

方針:

- 旧 schema を延命する migration は書かない
- 必要なら schema version を切り替えて新 DB として扱う

完了条件:

- store 単体テストで file 単位の read/write/delete/search が通ること
- etag mismatch の衝突が確認できること

### Phase 3: runtime / canister 差し替え

やること:

- `WikiService` を FS API 中心に差し替える
- canister entrypoint を FS API に置き換える
- `wiki.did` を更新する

完了条件:

- canister テストが新 API 前提で通ること
- migration 後に `read/list/write/search` が一貫して動くこと

### Phase 4: CLI / plugin 更新

やること:

- CLI を path ベース操作へ更新する
- plugin を node mirror に更新する
- pull/push/conflict を etag ベースに揃える

完了条件:

- local `Wiki/` と remote node の roundtrip が成立すること
- agent が file path ベースで自然に扱えること

### Phase 5: 旧実装削除

やること:

- wiki 固有 schema とコードを削除する
- README と計画書を FS-first 前提に更新する
- 不要テストを削除し、新モデルのテストに置き換える

完了条件:

- `page/revision/section/system page` 依存コードが残っていないこと

## 実装順

実装順は次で固定する。

1. `wiki_types` に新型を追加
2. `wiki.did` 新案を追加
3. `nodes` schema と migration を追加
4. store 実装と FTS 実装を追加
5. runtime を差し替え
6. canister を差し替え
7. CLI を更新
8. plugin を更新
9. 旧 wiki 実装を削除

## テスト方針

最低限必要なテストは次の通り。

- write 後に read できる
- list が prefix ごとに正しく返る
- delete 後に read/search に出ない
- search が current content だけを見る
- stale etag の write/delete が失敗する
- snapshot/export/fetch_updates が path 単位で整合する
- CLI pull/push の roundtrip が崩れない

## 主な設計判断

初期案として以下を採用する。

- path は absolute-like な `/Wiki/...` 文字列
- rename は初期版では未対応
- binary は初期版では未対応
- delete は tombstone を残す
- search は全文検索のみ
- history は持たない
- index/log は agent が普通の file として管理する

## リスク

### 失うもの

- revision 履歴の明示性
- section 単位の差分と競合解決
- system page の自動整合
- wiki 構造を DB が担保する仕組み

### 得るもの

- agent にとって自然な操作体系
- API の単純化
- local mirror と remote の対応の明確化
- wiki 以外の agent memory にも展開しやすい構造

## 判断

この移行は「wiki を薄くする」のではなく、プロダクトの中心を wiki model から agent filesystem model に置き換える判断である。

そのため、中途半端に両モデルを共存させず、FS-first を正本として一気に切り替える方が設計は安定する。
