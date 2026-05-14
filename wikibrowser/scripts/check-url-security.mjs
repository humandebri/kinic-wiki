import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const sourceExtractRoute = readFileSync(new URL("../app/api/sources/extract/route.ts", import.meta.url), "utf8");
const wikiBrowser = readFileSync(new URL("../components/wiki-browser.tsx", import.meta.url), "utf8");
const documentPane = readFileSync(new URL("../components/document-pane.tsx", import.meta.url), "utf8");
const routeModule = await importTs("../app/api/sources/extract/route.ts");
const triggerRouteModule = await importTs("../app/api/url-ingest/trigger/route.ts");

assert.match(sourceExtractRoute, /redirect: "manual"/);
assert.match(sourceExtractRoute, /MAX_REDIRECTS = 5/);
assert.match(sourceExtractRoute, /new URL\(location, currentUrl\.toString\(\)\)/);
assert.match(sourceExtractRoute, /normalized\.startsWith\("\["\) \|\| normalized\.includes\(":"\)/);
assert.match(sourceExtractRoute, /first === 127/);
assert.match(sourceExtractRoute, /first === 169 && second === 254/);

assert.doesNotMatch(wikiBrowser, /onLogin=\{login\}[\s\S]{0,140}<TopBar/);
assert.match(wikiBrowser, /authPromptMode\(tab, readIdentity, currentNode\.error \|\| currentChildren\.error\)/);
assert.match(wikiBrowser, /tab === "ingest" \|\| tab === "sources"/);
assert.match(documentPane, /authPrompt\?: "private" \| "write" \| null/);
assert.match(documentPane, /Write access/);

await withMockFetch(async () => new Response(null, { status: 302, headers: { location: "http://127.0.0.1/private" } }), async () => {
  const response = await routeModule.POST(jsonRequest("https://example.com/source"));
  assert.equal(response.status, 502);
});

await withMockFetch(async (input, init) => {
  assert.equal(init?.redirect, "manual");
  if (inputUrl(input) === "https://example.com/source") {
    return new Response(null, { status: 302, headers: { location: "/final#fragment" } });
  }
  return new Response("<html><body>Source</body></html>", { status: 200, headers: { "content-type": "text/html" } });
}, async () => {
  const response = await routeModule.POST(jsonRequest("https://example.com/source"));
  const body = await response.json();
  assert.equal(response.status, 200);
  assert.equal(body.url, "https://example.com/final");
});

await withEnv({}, async () => {
  const response = await triggerRouteModule.POST(triggerRequest("https://wiki.kinic.xyz"));
  assert.equal(response.status, 503);
  assert.match(await response.text(), /KINIC_WIKI_GENERATOR_URL is not configured/);
});

await withEnv(
  {
    NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID: "aaaaa-aa",
    KINIC_WIKI_GENERATOR_URL: "https://worker.example",
    KINIC_WIKI_WORKER_TOKEN: "secret-token"
  },
  async () => {
    const forbidden = await triggerRouteModule.POST(triggerRequest("https://evil.example"));
    assert.equal(forbidden.status, 403);

    const preflight = triggerRouteModule.OPTIONS(triggerRequest("chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj"));
    assert.equal(preflight.status, 204);
    assert.equal(preflight.headers.get("access-control-allow-origin"), "chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj");

    const invalidPath = await triggerRouteModule.POST(
      triggerRequest("https://kinic.xyz", { requestPath: "/Sources/raw/1.md" })
    );
    assert.equal(invalidPath.status, 400);

    const missingSessionNonce = await triggerRouteModule.POST(
      triggerRequest("https://kinic.xyz", { sessionNonce: "" })
    );
    assert.equal(missingSessionNonce.status, 400);

    const missingCanisterId = await triggerRouteModule.POST(
      triggerRequest("https://kinic.xyz", { canisterId: "" })
    );
    assert.equal(missingCanisterId.status, 400);

    const mismatchedCanisterId = await triggerRouteModule.POST(
      triggerRequest("https://kinic.xyz", { canisterId: "bbbbb-bb" })
    );
    assert.equal(mismatchedCanisterId.status, 400);

    triggerRouteModule.setUrlIngestTriggerDepsForTest({
      checkSession: async () => {
        throw new Error("denied");
      }
    });
    await withMockFetch(async () => {
      throw new Error("worker should not be called");
    }, async () => {
      const response = await triggerRouteModule.POST(triggerRequest("https://wiki.kinic.xyz"));
      assert.equal(response.status, 403);
    });

    triggerRouteModule.setUrlIngestTriggerDepsForTest({
      checkSession: async (canisterId, input) => {
        assert.equal(canisterId, "aaaaa-aa");
        assert.deepEqual(input, {
          canisterId: "aaaaa-aa",
          databaseId: "db_1",
          requestPath: "/Sources/ingest-requests/1.md",
          sessionNonce: "session-1"
        });
      }
    });
    await withMockFetch(async (input, init) => {
      assert.equal(inputUrl(input), "https://worker.example/url-ingest");
      assert.equal(init?.headers?.authorization, "Bearer secret-token");
      assert.equal(init?.method, "POST");
      assert.deepEqual(JSON.parse(init?.body), {
        canisterId: "aaaaa-aa",
        databaseId: "db_1",
        requestPath: "/Sources/ingest-requests/1.md"
      });
      return Response.json({ accepted: true }, { status: 202 });
    }, async () => {
      const response = await triggerRouteModule.POST(triggerRequest("https://wiki.kinic.xyz"));
      assert.equal(response.status, 200);
      assert.equal(response.headers.get("access-control-allow-origin"), "https://wiki.kinic.xyz");
    });
    triggerRouteModule.setUrlIngestTriggerDepsForTest();
  }
);

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

async function withEnv(values, run) {
  const keys = ["NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID", "KINIC_WIKI_CANISTER_ID", "KINIC_WIKI_GENERATOR_URL", "KINIC_WIKI_WORKER_TOKEN"];
  const previous = Object.fromEntries(keys.map((key) => [key, process.env[key]]));
  for (const key of keys) delete process.env[key];
  Object.assign(process.env, values);
  try {
    await run();
  } finally {
    for (const key of keys) {
      if (previous[key] === undefined) delete process.env[key];
      else process.env[key] = previous[key];
    }
  }
}

function jsonRequest(url) {
  return new Request("https://local.test/api/sources/extract", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ url })
  });
}

function triggerRequest(origin, overrides = {}) {
  return new Request("https://local.test/api/url-ingest/trigger", {
    method: "POST",
    headers: { "content-type": "application/json", origin },
    body: JSON.stringify({
      canisterId: "aaaaa-aa",
      databaseId: "db_1",
      requestPath: "/Sources/ingest-requests/1.md",
      sessionNonce: "session-1",
      ...overrides
    })
  });
}

function inputUrl(input) {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.toString();
  return input.url;
}
