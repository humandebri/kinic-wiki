# Ingest Workflow

## Goal

Turn raw source material into review-ready wiki updates under the canister-backed llm-wiki model.

## Workflow

1. Inspect the source material and the user focus.
2. If the source is noisy web or PDF-derived text, normalize it first.
3. Decide whether the source should also be persisted under `/Sources/raw/...`.
4. Read existing wiki context with `read`, `search`, `search_paths`, `recent`, or their CLI equivalents.
5. Choose the minimum coherent set of pages to update.
6. Edit `/Wiki/...` directly through raw tools or `wiki-cli` remote VFS commands.
7. Run `rebuild-index` only when durable wiki structure changed enough that `index.md` is stale.
8. Stop at review-ready unless the user explicitly asks for push.

## Working Rules

- Treat local `Wiki/` content as the human review surface.
- Prefer fewer stronger pages over many shallow stubs.
- Reuse existing pages when possible instead of minting near-duplicates.
- Do not hide push behind ingest.

## Repo Contract

- Raw source write path: `/Sources/raw/<source_id>/<source_id>.md`
- Raw source append path: `/Sources/raw/<source_id>/<source_id>.md`
- Wiki target root: `/Wiki/...`
- Preferred primitives:
  - agent tools: `read`, `write`, `append`, `edit`, `ls`, `glob`, `recent`, `search`, `search_paths`
  - CLI commands: `read-node`, `write-node`, `append-node`, `edit-node`, `list-nodes`, `glob-nodes`, `recent-nodes`, `search-remote`, `search-path-remote`, `rebuild-index`

## Output

Prefer one of these outputs:

- review-ready wiki page updates
- a page map and update plan before writing
- persisted raw source plus linked wiki updates

When useful, also provide:

- pages created or updated
- source files used
- open questions that block push
