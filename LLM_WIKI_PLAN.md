# LLM Wiki 計画書

## 1. 目的

このプロジェクトの目的は、raw source を毎回 RAG するのではなく、LLM が継続的に保守する永続 wiki を ICP 上に構築することです。

初期段階では、以下を重視します。

- wiki の正本を安定して保持できること
- index.md と lexical search だけで中規模まで回ること
- 後から vector retrieval を追加できること
- full rebuild ではなく差分更新できること

## 2. 採用判断

`ic-hybrid-sqlite` は採用します。
ただし、wiki システム本体ではなく retrieval subsystem として使います。

理由:

- README 上の責務が明確で、検索エンジンとしての境界が良い
- FTS5 keyword retrieval、sqlite-vec vector retrieval、fusion、migration、consistency check がすでにある
- 一方で chunking、embedding 生成、queue/worker、LLM 連携はスコープ外で、wiki 本体の責務と分離しやすい
- 現状 API は insert/search 中心で、差分更新に必要な stable key ベースの upsert/delete が不足している

結論:

- 正本: app 独自の wiki tables
- 検索: `ic-hybrid-sqlite` の `documents` を projection として利用
- 初期: lexical only
- 後期: current wiki sections のみ vector 化

## 3. 全体アーキテクチャ

3 層で構成します。

### 3.1 Raw Sources

不変の一次情報です。

- article
- paper
- transcript
- image metadata
- 外部メモ

raw source 自体は原則書き換えません。

### 3.2 Wiki

LLM が保守する markdown 的な知識層です。

- entity page
- concept page
- overview
- comparison
- query note
- system pages (`index.md`, `log.md`)

ユーザーは主にこの層を読みます。

### 3.3 Schema / Runtime Rules

LLM がどう wiki を更新するかを決める運用ルールです。

- ページ種別
- citation 規約
- ingest/query/lint の流れ
- page 更新ルール
- index/log の生成ルール

## 4. モジュール構成

初期構成は以下を想定します。

```text
workspace/
  crates/
    wiki_types/
    wiki_store/
    wiki_search/
    wiki_runtime/
    wiki_agent_schema/
```

### 4.1 `wiki_types`

純粋な型定義です。

- `Source`
- `SourceChunk`
- `WikiPage`
- `WikiRevision`
- `WikiSection`
- `Citation`
- `LogEvent`
- `SearchProjectionDoc`

### 4.2 `wiki_store`

正本 DB を扱う層です。

役割:

- source 管理
- page/revision/section 管理
- system page render
- log 追記
- citation 管理
- jobs 管理

### 4.3 `wiki_search`

`ic-hybrid-sqlite` fork を隠す検索層です。

役割:

- projection doc の upsert/delete
- lexical search
- hybrid search
- search result の page/section 集約

### 4.4 `wiki_runtime`

ICP canister entrypoint です。

役割:

- query/update API の公開
- payload 制約の管理
- store と search の調停

### 4.5 `wiki_agent_schema`

LLM 用の運用規約をまとめる層です。

役割:

- ingest 手順
- query 結果の file back ルール
- lint 観点
- page テンプレート

## 5. 正本スキーマ方針

検索用 `documents` を正本にしません。
正本は app tables に分離します。

最低限のテーブル:

- `wiki_pages`
- `wiki_revisions`
- `wiki_sections`
- `sources`
- `source_chunks`
- `revision_citations`
- `log_events`
- `system_pages`
- `jobs`

設計意図:

- `wiki_pages` は logical page の安定 ID を持つ
- `wiki_revisions` は履歴を持つ
- `wiki_sections` は検索・差分更新の最小単位
- `system_pages` は `index.md` / `log.md` の materialized view
- `jobs` は embedding などの非同期作業を管理

## 6. Projection 方針

検索対象は page 単位ではなく section 単位にします。

理由:

- BM25 が効きやすい
- wiki-only embedding を入れやすい
- section hash 単位で差分更新できる

検索 projection の logical key は以下です。

```text
page:{page_id}:section:{section_path}
page:{page_id}:index
page:{page_id}:query_note:{section_path}
sys:index.md
sys:index/entities.md
sys:log.md
```

`revision_id` は projection key に含めません。
current wiki 用 projection と割り切ります。

## 7. `ic-hybrid-sqlite` に追加する最小 fork

差分更新のために、以下を追加します。

### 7.1 schema 拡張

`documents` に以下を追加します。

- `external_id TEXT`
- `kind TEXT NOT NULL DEFAULT 'wiki_section'`
- `updated_at INTEGER NOT NULL DEFAULT 0`

index:

- `UNIQUE INDEX idx_documents_external_id`
- `INDEX idx_documents_kind_version_section_id`
- `INDEX idx_documents_updated_at`

### 7.2 API 拡張

最低限の追加 API:

- `upsert_document_by_external_id`
- `delete_document_by_external_id`
- `delete_documents_by_prefix`

### 7.3 filter 拡張

現状 filter は `section` と `tags` のみなので、`kind` filter を追加します。

想定:

- `index_page`
- `wiki_section`
- `query_note`
- `system_log`

## 8. index.md / log.md 方針

### 8.1 index.md

`index.md` は手編集しません。
`wiki_pages` から render して `system_pages` に保存します。

初期は 1 枚で運用し、規模が増えたら shard します。

候補:

- `index.md`
- `index/entities.md`
- `index/concepts.md`
- `index/sources.md`
- `index/queries.md`

役割は「最初に読むナビ」です。

### 8.2 log.md

`log.md` も正本ではなく render view とします。

正本:

- `log_events`

render 形式:

```text
## [2026-04-06] ingest | Article Title
## [2026-04-06] query | comparison of x and y
## [2026-04-06] lint | orphan pages
```

## 9. 検索戦略

### Phase 1

vector は使いません。

検索手順:

1. `index.md` または shard を読む
2. exact title / slug / tag match
3. FTS5 BM25 で `wiki_section` / `index_page` を引く
4. page 単位に集約
5. top page の current revision を読む
6. 必要なら citation から raw source に飛ぶ

投入対象:

- `index.md`
- current wiki sections
- query note
- comparison
- overview
- source summary

投入しない対象:

- raw source chunks 全量
- old revisions
- orphan draft pages

### Phase 2

current wiki sections だけ vector 化します。

方針:

- raw docs 全量 embedding はやらない
- index page は無理に vector 化しない
- hybrid は lexical shortlist の補助として使う

### Phase 3

必要なら raw の summary 系だけ補助投入します。

- source summary
- canonical digest

ここでも raw chunks 全量 vector 化は避けます。

## 10. 差分更新方針

差分単位は section hash です。

```text
content_hash = sha256(normalized_section_text)
```

revision 更新時の流れ:

1. markdown を section 分割
2. 各 section の `content_hash` を計算
3. 旧 current sections と比較
4. unchanged は何もしない
5. changed/new は projection を upsert
6. removed は projection を delete
7. embedding は changed/new だけ再計算

full rebuild はしません。

## 11. ICP canister API 方針

canister は wiki application service として切ります。
`ic_hybrid_runtime` をそのまま app 本体にはしません。

### query

- `get_page(slug)`
- `get_system_page(slug)`
- `search_lexical(req)`
- `search_hybrid(req)`
- `get_recent_log(limit)`
- `get_page_sources(page_id)`

### update

- `create_source(input)`
- `append_source_chunk(input)`
- `finalize_source(source_id)`
- `commit_page_revision(input)`
- `render_system_pages()`
- `enqueue_embedding_jobs(page_id)`
- `run_job_step(job_id)`

大きい raw docs は chunk upload 前提にします。

## 12. GitHub 風運用への発展

最終的には「ローカル編集者が push し、閲覧者が CLI/MCP で読む」モデルに発展させます。

ただし初期段階では、まず wiki canister 単体を完成させます。

順序:

1. ICP 上の wiki 正本と検索を成立させる
2. revision / projection / system pages の更新を安定化する
3. その後で push/pull 相当の同期モデルを設計する

理由:

- 先に GitHub 風同期を作ると、正本モデルが固まる前に外部契約が固まってしまう
- まずは単一 canister 内で整合した wiki runtime を作る方が安全

## 13. 実装フェーズ

### Phase A: 正本と lexical retrieval

やること:

- app schema 実装
- page revision / section split
- `index.md` render
- `log.md` render
- lexical projection
- exact + BM25 search
- query note file back

完了条件:

- current wiki sections を lexical 検索できる
- index/log が自動更新される
- page revision 更新で projection が追随する

### Phase B: search engine fork

やること:

- `external_id`
- `kind`
- `updated_at`
- upsert/delete API
- `kind` filter
- diff refresh の接続

完了条件:

- full rebuild なしで projection を差分更新できる
- remove section が search index から確実に消える

### Phase C: wiki-only embeddings

やること:

- changed sections のみ embedding enqueue
- hybrid retrieval
- lexical shortlist + vector shortlist + fusion

完了条件:

- vector 導入後も raw docs 全量埋め込みなしで運用できる
- changed section だけ再埋め込みされる

### Phase D: lint / health check

やること:

- contradiction tracking
- stale claim detection
- orphan page detection
- unsupported claim detection

完了条件:

- wiki の保守作業を LLM が継続的に提案できる

## 14. やらないこと

初期にやらないことを明確に固定します。

- `documents` を wiki 正本にする
- raw chunks 全量を最初から vector 化する
- old revisions を検索対象にする
- page 単位で毎回再埋め込みする
- `index.md` を手で保守する
- queue/worker を search engine 側に押し込む

## 15. 直近の実装順

最初の着手順は以下です。

1. `wiki_types` の型を固定する
2. app schema を migration として定義する
3. `commit_page_revision` の入出力を固定する
4. markdown -> sections 変換と `content_hash` を実装する
5. `system_pages` の render を入れる
6. lexical projection を接続する
7. その後で `ic-hybrid-sqlite` の fork 変更に入る

## 16. ライブラリ修正の実装計画

ここでは `ic-hybrid-sqlite` を wiki 用 retrieval engine として成立させるための最小変更を定義します。

### 16.1 変更方針

方針は「既存の insert/search を壊さずに、stable key ベースの更新能力を追加する」です。

守ること:

- 既存の `insert_document` / `insert_documents` は維持する
- 既存の lexical/hybrid ranking は基本維持する
- migration は追加方式で行う
- wiki 用途に必要な列と API だけ足す

### 16.2 変更対象

対象 crate:

- `ic_hybrid_types`
- `ic_hybrid_engine`
- `ic_hybrid_runtime`

主な修正ファイル:

- `crates/ic_hybrid_types/src/lib.rs`
- `crates/ic_hybrid_engine/src/lib.rs`
- `crates/ic_hybrid_engine/src/document.rs`
- `crates/ic_hybrid_engine/src/query.rs`
- `crates/ic_hybrid_engine/src/schema.rs`
- `crates/ic_hybrid_engine/migrations/*.sql`
- `crates/ic_hybrid_runtime/src/service.rs`
- `crates/ic_hybrid_runtime/src/canister.rs`

### 16.3 ステップ 1: 型拡張

目的:

- logical key と検索種別を API 契約に出す

追加候補:

- `IndexedDocument.external_id: Option<String>`
- `IndexedDocument.kind: Option<String>`
- `IndexedDocument.updated_at: Option<i64>`
- `SearchDocument.external_id: Option<String>`
- `SearchDocument.kind: Option<String>`
- `HybridQueryFilters.kinds: Vec<String>`

判断:

- 後方互換優先なら `Option`
- wiki 用の内部利用を優先するなら engine 内部で required 扱い

初期実装では wire 互換を保つため `Option` で入れ、wiki 側で必須にします。

### 16.4 ステップ 2: migration 追加

目的:

- `documents` に stable key と分類情報を持たせる

追加 migration:

- `002_external_document_keys.sql`

内容:

- `ALTER TABLE documents ADD COLUMN external_id TEXT`
- `ALTER TABLE documents ADD COLUMN kind TEXT NOT NULL DEFAULT 'wiki_section'`
- `ALTER TABLE documents ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0`
- `CREATE UNIQUE INDEX ... external_id`
- `CREATE INDEX ... kind, version, section, id`
- `CREATE INDEX ... updated_at`

併せて `schema.rs` の更新が必要です。

更新内容:

- `REQUIRED_DOCUMENT_COLUMNS` に新列を反映
- legacy 判定を新スキーマ基準に寄せる
- derived rebuild が新列追加後も壊れないことを確認

### 16.5 ステップ 3: document 操作の抽象化

目的:

- insert/update/delete で FTS/tag/vector を一貫更新する

`document.rs` で整理する内容:

- `insert_indexed_document_in_tx`
- `upsert_document_by_external_id_in_tx`
- `delete_document_by_external_id_in_tx`
- `delete_documents_by_prefix_in_tx`
- `replace_document_indexes_in_tx`
- `delete_document_indexes_in_tx`

要点:

- `documents`
- `documents_fts`
- `document_tags`
- vector index

この 4 つを必ず同一 transaction で扱います。

### 16.6 ステップ 4: public API 追加

`HybridEngine` に以下を追加します。

- `upsert_document_by_external_id`
- `delete_document_by_external_id`
- `delete_documents_by_prefix`

runtime にも必要なら同名メソッドを追加します。

ただし canister entrypoint は、wiki app canister を別で作る前提なら無理に広げなくてよいです。

### 16.7 ステップ 5: query filter 拡張

目的:

- `index_page`、`wiki_section`、`query_note` を検索時に分けられるようにする

変更内容:

- `HybridQueryFilters` に `kinds: Vec<String>` を追加
- lexical candidate query に `kind` filter を追加
- fallback candidate query に `kind` filter を追加
- vector candidate 後の document filter に `kind` filter を追加

`kind` の semantics は OR にします。

例:

- `["index_page", "wiki_section"]` は両方許可

### 16.8 ステップ 6: テスト追加

最低限の追加テスト:

- same `external_id` の upsert で置換される
- upsert 後に FTS hit が新内容へ切り替わる
- upsert 後に tags が古い値から更新される
- delete で FTS/tag/vector が全て消える
- prefix delete で複数 section が消える
- `kind` filter が lexical/hybrid/fallback で効く
- migration 後も既存 insert/search が動く

### 16.9 実装順

ライブラリ修正の具体的な順序:

1. `ic_hybrid_types` 拡張
2. migration 追加
3. `schema.rs` 更新
4. `document.rs` に upsert/delete 実装
5. `lib.rs` に public API 追加
6. `query.rs` に `kind` filter 追加
7. engine test 追加
8. runtime surface は必要最小限で追従

## 17. API 契約

実装前に、wiki 側の API 契約を固定しておくべきです。

理由:

- section 差分更新の入出力がここで決まる
- search projection の key 設計がここで固定される
- canister/query/update の責務分離がここで固まる

結論:

- `commit_page_revision`
- `search_lexical`
- `search_hybrid`

この 3 つは先に固定した方がよいです。

### 17.1 `commit_page_revision`

役割:

- 新しい page revision を登録する
- markdown を section 分割する
- current revision を切り替える
- search projection を差分更新する
- 必要なら embedding job を積む

初版契約:

```rust
pub struct CommitPageRevisionInput {
    pub page_id: String,
    pub expected_current_revision_id: Option<String>,
    pub title: String,
    pub markdown: String,
    pub change_reason: String,
    pub author_type: String,
    pub citations: Vec<RevisionCitationInput>,
    pub tags: Vec<String>,
    pub updated_at: i64,
}
```

```rust
pub struct RevisionCitationInput {
    pub source_id: String,
    pub chunk_id: Option<String>,
    pub evidence_kind: String,
    pub note: Option<String>,
}
```

制約:

- `page_id` は既存 page を指す
- `markdown` は空不可
- `expected_current_revision_id` が不一致なら競合で失敗

出力契約:

```rust
pub struct CommitPageRevisionOutput {
    pub revision_id: String,
    pub revision_no: u64,
    pub section_count: u32,
    pub unchanged_section_count: u32,
    pub upserted_projection_ids: Vec<String>,
    pub deleted_projection_ids: Vec<String>,
    pub enqueued_job_ids: Vec<String>,
    pub rendered_system_pages: Vec<String>,
}
```

処理契約:

1. page 存在確認
2. revision 作成
3. markdown を section 分割
4. `content_hash` 計算
5. 旧 current sections と比較
6. `wiki_sections` 更新
7. search projection upsert/delete
8. `index.md` / `log.md` 再描画
9. changed section の embedding jobs を enqueue

### 17.2 `search_lexical`

役割:

- Phase 1 の主検索 API

初版契約:

```rust
pub struct LexicalSearchRequest {
    pub query_text: String,
    pub kinds: Vec<String>,
    pub page_ids: Vec<String>,
    pub tags: Vec<String>,
    pub section: Option<String>,
    pub top_k: u32,
}
```

出力契約:

```rust
pub struct SearchHit {
    pub external_id: String,
    pub kind: String,
    pub page_id: String,
    pub revision_id: String,
    pub section_path: String,
    pub title: String,
    pub snippet: String,
    pub citation: String,
    pub tags: Vec<String>,
    pub score: f32,
    pub match_reasons: Vec<String>,
}
```

挙動:

- exact title / slug / tag match は app 層で先に補助判定してよい
- その後 FTS BM25 を適用する
- ranking は page 集約前の section hit を返す
- page 集約は app 層で実施する
- `kinds` が空なら `wiki_section` と `index_page` を既定対象にする
- `page_ids` が空なら page 制限なし

初期の `kinds` 推奨値:

- `index_page`
- `wiki_section`
- `query_note`

### 17.3 `search_hybrid`

役割:

- Phase 2 以降の検索 API

初版契約:

```rust
pub struct HybridSearchRequest {
    pub query_text: String,
    pub query_embedding: Vec<f32>,
    pub kinds: Vec<String>,
    pub page_ids: Vec<String>,
    pub tags: Vec<String>,
    pub section: Option<String>,
    pub top_k: u32,
    pub keyword_candidate_limit: Option<u32>,
    pub vector_candidate_limit: Option<u32>,
}
```

出力契約:

- `SearchHit` と同一

挙動:

- lexical shortlist と vector shortlist を fuse
- raw source ではなく current wiki sections を対象にする
- embedding がない section は lexical 側だけで残れる
- `kinds` が空なら `wiki_section` のみを既定対象にする

### 17.4 `get_page`

検索結果の着地点として、この API も早めに固定してよいです。

初版契約:

```rust
pub struct GetPageRequest {
    pub slug: String,
}
```

出力契約:

```rust
pub struct PageBundle {
    pub page_id: String,
    pub slug: String,
    pub title: String,
    pub page_type: String,
    pub current_revision_id: String,
    pub markdown: String,
    pub sections: Vec<PageSectionView>,
    pub citations: Vec<PageCitationView>,
    pub updated_at: i64,
}
```

## 18. 未決事項

着手前にまだ決める必要がある点です。

- `page_type` の列挙
- `SearchDocKind` の列挙
- `section_path` の正規化ルール
- citation の最小粒度
- query note を通常 page に昇格させる基準
- 1 canister に載せるサイズ上限をどう見るか
- 将来の push model で conflict をどう扱うか

## 19. 現時点の結論

現時点の採用方針は以下です。

- `ic-hybrid-sqlite` は使う
- ただし retrieval subsystem として使う
- 正本は app tables に置く
- projection は current wiki sections のみ
- 初期は lexical only
- 後で wiki-only embeddings
- raw docs embedding は最後まで避ける
- 差分更新は section hash 単位

この方針で進めるのが、責務分離・拡張性・ICP 上の実装容易性のバランスが最も良いです。
