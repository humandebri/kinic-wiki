// Where: wikibrowser/scripts/check-aeo-parser.mjs
// What: Runtime contract checks for AEO Markdown parsing.
// Why: Existing canister Markdown may lack generator-only fields such as sources.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import ts from "typescript";

const require = createRequire(import.meta.url);
const root = process.cwd();
const sourcePath = path.join(root, "lib/aeo/parse-markdown.ts");
const source = fs.readFileSync(sourcePath, "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022
  }
});

const module = { exports: {} };
vm.runInNewContext(compiled.outputText, {
  exports: module.exports,
  module,
  require
});

const { parseAeoMarkdown } = module.exports;

function markdown(frontmatter) {
  return `---\n${frontmatter.trim()}\n---\n\nBody`;
}

const withSources = parseAeoMarkdown(
  markdown(`
title: What is Kinic?
description: Kinic publishes public memory answers.
answer_summary: Kinic publishes answer pages.
updated: 2026-05-07
index: true
entities:
  - Kinic
sources:
  - README.md
  - app/page.tsx
`)
);

assert.equal(withSources.frontmatter.sources.length, 2);
assert.equal(
  JSON.stringify(withSources.frontmatter.sources),
  JSON.stringify(["README.md", "app/page.tsx"])
);

const withoutSources = parseAeoMarkdown(
  markdown(`
title: What is Kinic?
description: Kinic publishes public memory answers.
answer_summary: Kinic publishes answer pages.
updated: 2026-05-07
index: true
`)
);

assert.equal(JSON.stringify(withoutSources.frontmatter.sources), JSON.stringify([]));

for (const invalid of [
  "description: Missing title\nanswer_summary: Summary\nupdated: 2026-05-07\nindex: true",
  "title: Missing description\nanswer_summary: Summary\nupdated: 2026-05-07\nindex: true",
  "title: Missing summary\ndescription: Description\nupdated: 2026-05-07\nindex: true",
  "title: Missing updated\ndescription: Description\nanswer_summary: Summary\nindex: true",
  "title: Not indexed\ndescription: Description\nanswer_summary: Summary\nupdated: 2026-05-07\nindex: false"
]) {
  assert.equal(parseAeoMarkdown(markdown(invalid)), null);
}

console.log("AEO parser checks passed");
