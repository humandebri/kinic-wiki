---
name: frontend-aeo-wiki
description: Generate AEO wiki and answer-page drafts from the user-visible surface of a frontend repository, especially Next.js App Router apps. Use when turning frontend routes, pages, metadata, UI copy, README, or public docs into Kinic Wiki Markdown and /answers Markdown.
---

# Frontend AEO Wiki

Use this skill when a frontend repository should become an AI-search-ready public wiki.

## Scope

Only document user-visible product behavior.

Include:

- Next.js App Router pages and layouts
- route metadata and page titles
- UI copy visible in components used by public pages
- README and public docs
- public API behavior only when surfaced in UI or public docs

Exclude:

- database schemas
- backend-only code and internal clients
- tests, fixtures, generated files, build scripts
- secrets, environment values, private config
- hidden admin surfaces
- claims about pricing, security, compliance, performance, competitors, or roadmap unless explicitly present in visible UI, metadata, README, or public docs

## Workflow

1. Detect the frontend framework. For now, support Next.js App Router first.
2. Build a source pack from visible routes, layouts, metadata, README, and docs.
3. Generate concise Wiki Markdown:
   - `/Wiki/product/overview.md`
   - `/Wiki/product/screens.md`
   - `/Wiki/product/features.md`
   - `/Wiki/product/faq.md`
4. Generate AEO answer Markdown under `/Wiki/aeo/...`.
5. Every answer must include frontmatter with `title`, `description`, `answer_summary`, `updated`, `index: true`, `entities`, and `sources`.
6. Every answer body must cite repo-relative source paths.
7. Reject or omit unsupported claims instead of inventing product promises.

## Answer Contract

Use this frontmatter shape:

```md
---
title: What is Example?
description: Example helps users understand the visible product value.
answer_summary: Example provides the visible product experience described in the frontend.
updated: 2026-05-07
index: true
entities:
  - Example
sources:
  - README.md
  - app/page.tsx
---
```

The body should be short, factual, and grounded in `sources`.

## Validation

Before publishing, ensure:

- slugs are unique
- required frontmatter exists
- `sources` is non-empty
- Markdown parses
- no secret patterns appear
- no unsupported claim category appears without a visible source

If validation fails, do not publish generated pages.
