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
const { hrefForPath, pathFromSegments } = await import(moduleUrl);

assert.equal(pathFromSegments([]), "/Wiki");
assert.equal(pathFromSegments(["Wiki", "100%.md"]), "/Wiki/100%.md");
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "/Wiki/100%.md"),
  "/site/t63gs-up777-77776-aaaba-cai/Wiki/100%25.md"
);
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "/Wiki/space name.md", "raw"),
  "/site/t63gs-up777-77776-aaaba-cai/Wiki/space%20name.md?view=raw"
);
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "/Wiki/demo.md", undefined, "search"),
  "/site/t63gs-up777-77776-aaaba-cai/Wiki/demo.md?tab=search"
);
assert.equal(
  hrefForPath("t63gs-up777-77776-aaaba-cai", "/Wiki/demo.md", undefined, "search", "alpha beta"),
  "/site/t63gs-up777-77776-aaaba-cai/Wiki/demo.md?tab=search&q=alpha+beta"
);

console.log(`Path helpers OK: ${pathToFileURL(sourcePath.pathname).pathname}`);
