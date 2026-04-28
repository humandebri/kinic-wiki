import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const { classifyApiError, invalidCanisterIdError } = await importTs("../lib/api-errors.ts");

assert.equal(
  classifyApiError(new Error("Canister t63gs-up777-77776-aaaba-cai not found"), "http://127.0.0.1:8000").code,
  "canister_not_found"
);
assert.equal(
  classifyApiError(new Error("fetch failed: connect ECONNREFUSED 127.0.0.1:8000"), "http://127.0.0.1:8000").code,
  "ic_host_unreachable"
);
assert.equal(
  classifyApiError(new Error("Canister has no query method search_nodes"), "https://icp0.io").code,
  "wiki_api_missing"
);
assert.equal(classifyApiError(new Error("replica rejected request"), "https://icp0.io").error, "Wiki request failed");
assert.equal(invalidCanisterIdError("invalid principal").error, "Invalid canister ID");

console.log("API error checks OK");

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
