import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const { collectLintHints, provenancePathFor, rawSourceLinksFor } = await importTs("../lib/lint-hints.ts");
const { normalizeSearchHit } = await importTs("../lib/search-normalizer.ts");
const { readBrowserNodeCache } = await importTs("../lib/browser-node-cache.ts");
const { sortChildNodes } = await importTs("../lib/child-sort.ts");
const { cycleTone, formatCycles, formatRawCycles } = await importTs("../lib/cycles.ts");
const { splitMarkdownPreviewSections } = await importTs("../lib/markdown-sections.ts");
const { graphRequestKey, nodeRequestKey, searchRequestKey } = await importTs("../lib/request-keys.ts");
const { canExpandChildNode } = await importTs("../lib/wiki-helpers.ts");
const { buildSourceClipDocument, normalizeClipUrl, parseTags, sourceClipPath, renderSourceClipMarkdown } = await importTs("../lib/source-clips.ts");
const explorerTreeSource = readFileSync(new URL("../components/explorer-tree.tsx", import.meta.url), "utf8");
const searchPanelSource = readFileSync(new URL("../components/search-panel.tsx", import.meta.url), "utf8");
const wikiBrowserSource = readFileSync(new URL("../components/wiki-browser.tsx", import.meta.url), "utf8");
const globalsCss = readFileSync(new URL("../app/globals.css", import.meta.url), "utf8");

assert.match(explorerTreeSource, /childNodesCache\.current\.get\(requestKey\)/);
assert.match(explorerTreeSource, /childNodesCache\.current\.set\(requestKey, data\)/);
assert.match(explorerTreeSource, /key=\{`\$\{canisterId\}:\$\{databaseId\}:\/Wiki:/);
assert.match(wikiBrowserSource, /data-tid="header-login-button"/);
assert.match(wikiBrowserSource, /onClick=\{onLogin\}/);
assert.match(wikiBrowserSource, /LayoutDashboard/);
assert.match(wikiBrowserSource, /aria-label="Back to database dashboard"/);
assert.match(globalsCss, /button:not\(:disabled\):active/);
assert.match(globalsCss, /transform: scale\(0\.98\)/);
assert.match(globalsCss, /button\[aria-busy="true"\]/);
assert.match(globalsCss, /prefers-reduced-motion/);

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
assert.equal(canExpandChildNode(child("/Wiki/file-parent", "file-parent", "file", true)), true);
assert.equal(canExpandChildNode(child("/Wiki/file-leaf.md", "file-leaf.md", "file", false)), false);

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
assert.notEqual(nodeRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md"), nodeRequestKey("bbbbb-bb", "alpha", "/Wiki/index.md"));
assert.notEqual(
  graphRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md", 1),
  graphRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md", 2)
);
assert.equal(graphRequestKey("aaaaa-aa", "alpha", null, 1), null);
assert.equal(searchRequestKey("aaaaa-aa", "alpha", "path", "budget"), searchRequestKey("aaaaa-aa", "alpha", "path", "budget"));
assert.notEqual(searchRequestKey("aaaaa-aa", "alpha", "path", "budget"), searchRequestKey("aaaaa-aa", "alpha", "full", "budget"));
assert.notEqual(searchRequestKey("aaaaa-aa", "alpha", "path", "budget"), searchRequestKey("aaaaa-aa", "beta", "path", "budget"));
assert.notEqual(searchRequestKey("aaaaa-aa", "alpha", "path", "budget"), searchRequestKey("bbbbb-bb", "alpha", "path", "budget"));
assert.notEqual(nodeRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md"), nodeRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md", "aaaaa-aa"));
assert.notEqual(
  graphRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md", 1),
  graphRequestKey("aaaaa-aa", "alpha", "/Wiki/index.md", 1, "aaaaa-aa")
);
assert.notEqual(searchRequestKey("aaaaa-aa", "alpha", "path", "budget"), searchRequestKey("aaaaa-aa", "alpha", "path", "budget", "aaaaa-aa"));

const cachedNodeContext = {
  node: {
    path: "/Wiki/demo.md",
    kind: "file",
    content: "# Demo",
    updatedAt: null,
    etag: "node-etag",
    sizeBytes: 6
  },
  incomingLinks: [],
  outgoingLinks: []
};
const cachedChildren = [child("/Wiki/demo", "demo", "directory")];
const nodeContextCache = new Map([["node-key", cachedNodeContext]]);
const childNodesCache = new Map([["children-key", cachedChildren], ["node-key", cachedChildren]]);
assert.deepEqual(readBrowserNodeCache(nodeContextCache, childNodesCache, "missing-key"), null);
assert.deepEqual(readBrowserNodeCache(nodeContextCache, childNodesCache, "children-key"), {
  kind: "children",
  children: cachedChildren
});
assert.deepEqual(readBrowserNodeCache(nodeContextCache, childNodesCache, "node-key"), {
  kind: "node",
  context: cachedNodeContext
});

assert.equal(formatCycles(12_345_000_000_000n), "12.34T");
assert.equal(formatCycles(850_000_000_000n), "850.00B");
assert.equal(formatCycles(123_450_000n), "123.45M");
assert.equal(formatRawCycles(1234567890123n), "1,234,567,890,123");
assert.equal(cycleTone(5_000_000_000_000n), "blue");
assert.equal(cycleTone(1_000_000_000_000n), "amber");
assert.equal(cycleTone(999_999_999_999n), "red");
assert.equal(cycleTone(null), "gray");
assert.equal(normalizeClipUrl("HTTPS://Example.COM:443/a?b=1#section"), "https://example.com/a?b=1");
assert.throws(() => normalizeClipUrl("ftp://example.com/a"), /http or https/);
assert.deepEqual(parseTags("#easy, dinner easy\nquick"), ["easy", "dinner", "quick"]);
const clipPath = await sourceClipPath("https://example.com/a?b=1");
assert.match(clipPath, /^\/Sources\/raw\/clip-example\.com-[a-f0-9]{12}\/clip-example\.com-[a-f0-9]{12}\.md$/);
assert.equal(clipPath, await sourceClipPath("https://example.com/a?b=1"));
const clipSegments = clipPath.split("/");
assert.equal(clipSegments.at(-1), `${clipSegments.at(-2)}.md`);
const clipMarkdown = renderSourceClipMarkdown({
  url: "https://example.com/a",
  title: "Weeknight Pasta",
  site: "example.com",
  capturedAt: "2026-05-12T00:00:00.000Z",
  tags: ["easy", "pasta"],
  userNote: "halve salt",
  extractedText: "Ingredients\n- tomato"
});
assert.match(clipMarkdown, /source_url: "https:\/\/example\.com\/a"/);
assert.match(clipMarkdown, /# Weeknight Pasta/);
assert.match(clipMarkdown, /halve salt/);
const clipDocument = await buildSourceClipDocument({
  url: "https://example.com/a#ignored",
  title: "Weeknight Pasta",
  site: "example.com",
  capturedAt: "2026-05-12T00:00:00.000Z",
  tags: ["easy"],
  userNote: "",
  extractedText: "body"
});
assert.equal(clipDocument.normalizedUrl, "https://example.com/a");
assert.match(clipDocument.path, /^\/Sources\/raw\/clip-example\.com-[a-f0-9]{12}\/clip-example\.com-[a-f0-9]{12}\.md$/);
assert.equal(JSON.parse(clipDocument.metadataJson).app, "source_clip");
assert.match(searchPanelSource, /prefix = "\/Wiki"/);
assert.match(searchPanelSource, /prefix, readIdentity/);

console.log("UI helper checks OK");

function child(path, name, kind, hasChildren = kind === "directory") {
  return {
    path,
    name,
    kind,
    updatedAt: null,
    etag: null,
    sizeBytes: null,
    isVirtual: false,
    hasChildren
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
