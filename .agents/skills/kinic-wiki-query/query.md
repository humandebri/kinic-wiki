# Kinic Wiki Query Workflow

## Goal

Answer questions against the current wiki using CLI read and search commands.

## Workflow

1. Read `index.md` first when it exists for the current scope.
2. Choose the structured note whose role best matches the question shape, then read that note before broad search.
3. If the first role-matched note is empty or lacks the requested value, read the next canonical note directly before broad search.
4. Run `search-path-remote` or `search-remote` only when direct canonical-note reads are still missing, ambiguous, or insufficient.
5. Use `read-node`, with `recent-nodes` or `list-nodes` only when needed, to collect the minimum relevant note set.
6. Synthesize a source-backed answer from current wiki material.
7. If the user explicitly wants durable write-back, hand off to `kinic-wiki-ingest` instead of growing query-side mutation rules.

## Working Rules

- Current repo-local note schema lives in [docs/internal/WIKI_CANONICALITY.md](../../../docs/internal/WIKI_CANONICALITY.md). Treat that file as the source of truth for current note names and role mapping.
- Answer-shape rules live in [../references/query-rules.md](../references/query-rules.md). Use that file for abstention, extraction, and exact-value behavior.
- Prefer scope-first exploration.
- Once you open a conversation index or a note under one conversation path, try to finish inside that same conversation first.
- Within one conversation, start from `index.md`, then choose the structured note whose role best matches the question shape.
- For exact extraction or single-attribute questions, inspect the canonical note chain directly before any broad search.
- If `facts.md` is empty for an extraction question, move to the next role-matched note instead of returning `insufficient evidence` early.
- Do not return `insufficient evidence` while a higher-priority canonical note remains unread.
- Use `search-path-remote` and `search-remote` as targeted recall steps only after direct canonical-note reads are insufficient.
- Treat `search-path-remote` as path and basename recall.
- Treat `search-remote` as FTS-based content recall.
- If the question shape is still unclear after reading `index.md`, follow the current note roles from `docs/internal/WIKI_CANONICALITY.md` rather than inventing ad hoc search order.
- Return to broader search only after you fail to find direct evidence inside the current conversation scope.
- Do not answer from an index, list, or search result alone.
- Do not conclude absence until you have checked both path recall and content recall for the current scope.
- Before the final answer, read at least one note that directly supports the answer.
- Treat the final answer as invalid until it is anchored to a note you actually read.
- Treat `facts.md` as the first stop for stable attributes and exact extraction.
- Treat `events.md` as the first stop for chronology, order, and elapsed time.
- Treat `plans.md` as the first stop for directives, intended actions, and temporary constraints.
- Treat `preferences.md` as the first stop for preferences and recommendation style.
- Treat `open_questions.md` as the first stop for unresolved conflicts and contradiction questions.
- Treat `summary.md` as recap support for summary-style synthesis, not as the primary source for exact extraction.
- For multi-value extraction, preserve the requested slot order instead of collapsing multiple values into a generic summary.

## Repo Contract

- Preferred query primitives:
  - CLI commands: `read-node`, `list-nodes`, `search-remote`, `search-path-remote`, `recent-nodes`
