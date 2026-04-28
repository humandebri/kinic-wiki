import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const { collectLintHints } = await importTs("../lib/lint-hints.ts");
const { normalizeSearchHit } = await importTs("../lib/search-normalizer.ts");

const factsHints = collectLintHints("/Wiki/demo/facts.md", "Deadline is May 10.\nStable value is blue.");
assert.equal(factsHints.length, 1);
assert.equal(factsHints[0].title, "Possible future or pending item");

const summaryHints = collectLintHints("/Wiki/demo/summary.md", "Receipt AB-123456 was filed.");
assert.equal(summaryHints.length, 1);
assert.equal(summaryHints[0].title, "Possible exact evidence leak");

const codeHints = collectLintHints("/Wiki/demo/code-note.md", "- Implementation: `crates/vfs_store/src/fs_store.rs`");
assert.equal(codeHints.length, 1);
assert.equal(codeHints[0].title, "Code note lacks decision context");

const hit = normalizeSearchHit({
  path: "/Wiki/demo.md",
  kind: { File: null },
  snippet: ["demo snippet"],
  preview: [
    {
      field: { Content: null },
      char_offset: 42,
      match_reason: "content",
      excerpt: ["demo excerpt"]
    }
  ],
  score: 0.75,
  match_reasons: ["content"]
});
assert.deepEqual(hit, {
  path: "/Wiki/demo.md",
  kind: "file",
  snippet: "demo snippet",
  preview: {
    field: "content",
    charOffset: 42,
    matchReason: "content",
    excerpt: "demo excerpt"
  },
  score: 0.75,
  matchReasons: ["content"]
});

console.log("UI helper checks OK");

async function importTs(relativePath) {
  const sourcePath = new URL(relativePath, import.meta.url);
  const source = readFileSync(sourcePath, "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022
    }
  }).outputText;
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`;
  return import(moduleUrl);
}
