# Lint Workflow

## Goal

Inspect local and remote wiki health, report concrete findings, and propose the next repair action without silently applying fixes.

## Workflow

1. Decide whether the inspection target is local, remote, or both.
2. For local structure checks, use `wiki-cli lint-local`.
3. For remote checks, read `index.md`, inspect recent pages, and use `search`, `search_paths`, `ls`, `glob`, and `read`.
4. Group findings into:
   - duplication
   - isolation
   - stale navigation or index
   - missing cross-links
   - ambiguous page boundaries
5. Report findings first.
6. Only edit pages if the user asks for fixes or the workflow explicitly includes a repair step.

## Working Rules

- When `index.md` is stale, recommend or run `rebuild-index` as an explicit maintenance step.
- Keep local lint separate from remote content review.

## Repo Contract

- Local lint command: `wiki-cli lint-local --vault-path <path> [--json]`
- Remote inspection primitives:
  - agent tools: `read`, `ls`, `glob`, `recent`, `search`, `search_paths`
  - CLI commands: `read-node`, `list-nodes`, `glob-nodes`, `recent-nodes`, `search-remote`, `search-path-remote`, `rebuild-index`

## Output

Prefer:

- a prioritized findings list
- a short next-action plan

Optionally include:

- candidate page merges
- candidate missing links
- recommendation to rebuild `index.md`
