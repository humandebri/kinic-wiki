import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const { collectLintHints, provenancePathFor, rawSourceLinksFor } = await importTs("../lib/lint-hints.ts");
const { normalizeSearchHit } = await importTs("../lib/search-normalizer.ts");
const { sortChildNodes } = await importTs("../lib/child-sort.ts");
const { splitMarkdownPreviewSections } = await importTs("../lib/markdown-sections.ts");
const { graphRequestKey, nodeRequestKey } = await importTs("../lib/request-keys.ts");

const factsHints = collectLintHints("/Wiki/demo/facts.md", "Deadline is May 10.\nStable value is blue.");
assert.equal(factsHints.length, 1);
assert.equal(factsHints[0].title, "Possible future or pending item");
assert.equal(factsHints[0].preview, "Deadline is May 10.");

const summaryHints = collectLintHints("/Wiki/demo/summary.md", "Receipt AB-123456 was filed.");
assert.equal(summaryHints.length, 1);
assert.equal(summaryHints[0].title, "Possible exact evidence leak");

const codeHints = collectLintHints("/Wiki/demo/code-note.md", "- Implementation: `crates/vfs_store/src/fs_store.rs`");
assert.equal(codeHints.length, 1);
assert.equal(codeHints[0].title, "Code note lacks decision context");
assert.equal(codeHints[0].preview, "- Implementation: `crates/vfs_store/src/fs_store.rs`");

assert.deepEqual(
  rawSourceLinksFor("/Wiki/demo/provenance.md", "- Raw: /Sources/raw/demo/source.md\n- Raw: /Sources/raw/demo/source.md"),
  ["/Sources/raw/demo/source.md"]
);
assert.deepEqual(
  rawSourceLinksFor("/Sources/raw/demo/source.md", "# Raw"),
  ["/Sources/raw/demo/source.md"]
);
assert.equal(provenancePathFor("/Wiki/demo/facts.md"), "/Wiki/demo/provenance.md");
assert.equal(provenancePathFor("/Wiki/demo/provenance.md"), null);

const sortedChildren = sortChildNodes([
  child("/Wiki/10.md", "10.md", "file"),
  child("/Wiki/2.md", "2.md", "file"),
  child("/Wiki/beta", "beta", "directory"),
  child("/Wiki/1.md", "1.md", "file"),
  child("/Wiki/alpha", "alpha", "directory")
]);
assert.deepEqual(
  sortedChildren.map((node) => node.path),
  ["/Wiki/alpha", "/Wiki/beta", "/Wiki/1.md", "/Wiki/2.md", "/Wiki/10.md"]
);

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

assert.deepEqual(
  splitMarkdownPreviewSections("Intro\n\n# One\nBody\n## Two\nMore").map((section) => section.split("\n")[0]),
  ["Intro", "# One", "## Two"]
);
assert.deepEqual(
  splitMarkdownPreviewSections("# One\n```md\n# Not heading\n```\n## Two").map((section) => section.split("\n")[0]),
  ["# One", "## Two"]
);
assert.deepEqual(
  splitMarkdownPreviewSections("# One\n~~~md\n# Not heading\n~~~\n## Two").map((section) => section.split("\n")[0]),
  ["# One", "## Two"]
);
assert.equal(splitMarkdownPreviewSections("No headings\nOnly prose").length, 1);
assert.notEqual(nodeRequestKey("aaaaa-aa", "/Wiki/index.md"), nodeRequestKey("bbbbb-bb", "/Wiki/index.md"));
assert.notEqual(
  graphRequestKey("aaaaa-aa", "/Wiki/index.md", 1),
  graphRequestKey("aaaaa-aa", "/Wiki/index.md", 2)
);
assert.equal(graphRequestKey("aaaaa-aa", null, 1), null);

console.log("UI helper checks OK");

function child(path, name, kind) {
  return {
    path,
    name,
    kind,
    updatedAt: null,
    etag: null,
    sizeBytes: null,
    isVirtual: false
  };
}

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
