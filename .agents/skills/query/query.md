# Query Workflow

## Goal

Answer questions against the current wiki using raw read and search primitives, with optional durable write-back only when justified.

## Workflow

1. Start from `index.md` when it is likely to narrow the search.
2. Use `search`, `search_paths`, `recent`, `ls`, and `read` to collect the minimum relevant page set.
3. Synthesize a source-backed answer from current wiki material.
4. Only if the answer has durable reuse value, write a new or updated page under `/Wiki/...`.
5. Rebuild the index only when the page set changed enough to make `index.md` stale.

## Working Rules

- When writing back, prefer `comparison`, `query_note`, or synthesis pages only when they add durable value.
- Avoid turning every answer into content churn.

## Repo Contract

- Preferred query primitives:
  - agent tools: `read`, `ls`, `search`, `search_paths`, `recent`
  - CLI commands: `read-node`, `list-nodes`, `search-remote`, `search-path-remote`, `recent-nodes`
- Optional write-back primitives:
  - agent tools: `write`, `append`, `edit`, `multi_edit`
  - CLI commands: `write-node`, `append-node`, `edit-node`, `multi-edit-node`, `rebuild-index`

## Output

Prefer one of these outputs:

- a direct answer grounded in current wiki pages
- a comparison or synthesis summary
- an optional reusable page update under `/Wiki/...`

When writing back, include:

- why the result deserves durable storage
- which pages were created or updated
