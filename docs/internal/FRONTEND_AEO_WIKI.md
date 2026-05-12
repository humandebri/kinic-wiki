# Frontend AEO Wiki

Frontend AEO Wiki turns the user-visible surface of a frontend repo into Kinic Wiki pages and AEO answer pages.

## Product Flow

1. Developer connects a GitHub repository.
2. Developer pushes to `main`.
3. Kinic receives a push webhook.
4. Kinic analyzes the frontend surface.
5. Kinic-side LLM generation creates public Wiki and AEO answer drafts.
6. Mechanical validation gates publish.
7. Kinic publishes updated `/answers`, `sitemap.xml`, and `llms.txt`.

The target experience is Vercel-like: connect a repo, push to `main`, and let Kinic keep the AI-search-ready public knowledge layer current.

## Frontend Surface

MVP source inputs:

- Next.js App Router `app/**/page.tsx`
- Next.js App Router `app/**/layout.tsx`
- route metadata and visible UI copy
- root `README.md`
- public docs under `docs/**`

Excluded inputs:

- database schemas
- backend-only helpers and internal API clients
- tests and fixtures
- generated files and build scripts
- secrets, environment values, and private config
- hidden admin surfaces

Generated content must not claim pricing, security, compliance, performance, competitor superiority, or roadmap details unless the claim is explicit in visible UI, metadata, README, or public docs.

## Generated Output

Local dry-run output uses this shape:

```text
wiki/overview.md
wiki/screens.md
wiki/features.md
wiki/faq.md
answers/what-is-<project>.md
answers/how-does-<project>-work.md
answers/<project>-features.md
manifest.json
validation.json
```

Answer pages use the existing AEO frontmatter contract plus `sources`.

## Publish Contract

The current MVP only generates local artifacts. A later publish step can consume `manifest.json` to update Kinic Wiki nodes and derive `AeoPageConfig` allowlist entries.

Publish must pass mechanical validation:

- required frontmatter exists
- slug values are unique
- source paths are present
- Markdown parses
- no secret patterns are present
- generated claims are grounded in visible frontend or public docs
- `/w/*` remains `noindex`
- only `/answers/*` appears in `sitemap.xml` and `llms.txt`

If validation fails, Kinic keeps the previous published version and stores the failed generation report.
