import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const recipeExtractRoute = readFileSync(new URL("../app/api/recipes/extract/route.ts", import.meta.url), "utf8");
const wikiBrowser = readFileSync(new URL("../components/wiki-browser.tsx", import.meta.url), "utf8");
const documentPane = readFileSync(new URL("../components/document-pane.tsx", import.meta.url), "utf8");
const routeModule = await importTs("../app/api/recipes/extract/route.ts");

assert.match(recipeExtractRoute, /redirect: "manual"/);
assert.match(recipeExtractRoute, /MAX_REDIRECTS = 5/);
assert.match(recipeExtractRoute, /new URL\(location, currentUrl\.toString\(\)\)/);
assert.match(recipeExtractRoute, /normalized\.startsWith\("\["\) \|\| normalized\.includes\(":"\)/);
assert.match(recipeExtractRoute, /first === 127/);
assert.match(recipeExtractRoute, /first === 169 && second === 254/);

assert.doesNotMatch(wikiBrowser, /onLogin=\{login\}[\s\S]{0,140}<TopBar/);
assert.match(wikiBrowser, /authPromptMode\(tab, readIdentity, currentNode\.error \|\| currentChildren\.error\)/);
assert.match(wikiBrowser, /tab === "ingest" \|\| tab === "recipes"/);
assert.match(documentPane, /authPrompt\?: "private" \| "write" \| null/);
assert.match(documentPane, /Write access/);

await withMockFetch(async () => new Response(null, { status: 302, headers: { location: "http://127.0.0.1/private" } }), async () => {
  const response = await routeModule.POST(jsonRequest("https://example.com/recipe"));
  assert.equal(response.status, 502);
});

await withMockFetch(async (input, init) => {
  assert.equal(init?.redirect, "manual");
  if (inputUrl(input) === "https://example.com/recipe") {
    return new Response(null, { status: 302, headers: { location: "/final#fragment" } });
  }
  return new Response("<html><body>Recipe</body></html>", { status: 200, headers: { "content-type": "text/html" } });
}, async () => {
  const response = await routeModule.POST(jsonRequest("https://example.com/recipe"));
  const body = await response.json();
  assert.equal(response.status, 200);
  assert.equal(body.url, "https://example.com/final");
});

console.log("URL security checks OK");

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

async function withMockFetch(handler, run) {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = handler;
  try {
    await run();
  } finally {
    globalThis.fetch = originalFetch;
  }
}

function jsonRequest(url) {
  return new Request("https://local.test/api/recipes/extract", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ url })
  });
}

function inputUrl(input) {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.toString();
  return input.url;
}
