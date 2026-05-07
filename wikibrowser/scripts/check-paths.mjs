import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import ts from "typescript";

const sourcePath = new URL("../lib/paths.ts", import.meta.url);
const source = readFileSync(sourcePath, "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ES2022,
    target: ts.ScriptTarget.ES2022
  }
}).outputText;
const moduleUrl = `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`;
const { hrefForMarkdownLink, hrefForPath, hrefForSearch, pathFromSegments } = await import(moduleUrl);

assert.equal(pathFromSegments([]), "/Wiki");
assert.equal(pathFromSegments(["Wiki", "100%.md"]), "/Wiki/100%.md");
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "alpha", "/Wiki/100%.md"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/Wiki/100%25.md"
);
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "alpha", "/Wiki/space name.md", "raw"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/Wiki/space%20name.md?view=raw"
);
assert.equal(
  hrefForSearch("t63gs-up777-77776-aaaba-cai", "alpha", "", "path"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/search?kind=path"
);
assert.equal(
  hrefForSearch("t63gs-up777-77776-aaaba-cai", "alpha", "alpha beta", "path"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/search?q=alpha+beta&kind=path"
);
assert.equal(
  hrefForSearch("t63gs-up777-77776-aaaba-cai", "alpha", "alpha beta", "full"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/search?q=alpha+beta&kind=full"
);
assert.equal(
  hrefForMarkdownLink("t63gs-up777-77776-aaaba-cai", "alpha", "/Wiki/beam-full-reset/7/index.md", "facts.md"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/Wiki/beam-full-reset/7/facts.md"
);
assert.equal(
  hrefForMarkdownLink("t63gs-up777-77776-aaaba-cai", "alpha", "/Wiki/beam-full-reset/7/index.md", "/Wiki/demo.md#evidence"),
  "/w/t63gs-up777-77776-aaaba-cai/db/alpha/Wiki/demo.md#evidence"
);
assert.equal(
  hrefForMarkdownLink("t63gs-up777-77776-aaaba-cai", "alpha", "/Wiki/demo/index.md", "https://example.com"),
  null
);

console.log(`Path helpers OK: ${pathToFileURL(sourcePath.pathname).pathname}`);
