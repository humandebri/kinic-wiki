---
name: query
description: Query the llm-wiki knowledge base using raw search and read primitives. Use when answering questions against the current wiki, with optional promotion of high-value results back into `/Wiki/...`.
---

# Query

Use this skill when the user wants to:

- ask questions against the current wiki
- compare topics, entities, or concepts already represented in the wiki
- explore what the wiki currently knows before deciding on further ingestion
- optionally turn a high-value answer into a reusable page

Do not use this skill for:

- first-pass source ingestion
- health-only wiki inspection
- mandatory page creation for every answer

Core rules:

- Default to answer-only behavior.
- Do not force page creation for routine questions.
- Cite the wiki pages or source pages actually used.
- Keep the read set narrow and intentional.

Read [query.md](query.md) before doing substantive query work.

Read this reference when needed:

- shared mirror and markdown rules: [../wiki-generate/references/obsidian-rules.md](../wiki-generate/references/obsidian-rules.md)
