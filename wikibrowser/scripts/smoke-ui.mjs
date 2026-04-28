import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const url = readUrl();
const targetUrl = new URL(url);
const canisterId = targetUrl.pathname.split("/")[1];
const nodePath = `/${targetUrl.pathname.split("/").slice(2).join("/")}`;
const searchUrl = `${targetUrl.origin}/${encodeURIComponent(canisterId)}/search`;
const smokeWaitMs = 30_000;
const pollMs = 500;
const targetNode = await readTargetNode();
const contentProbe = contentProbeFor(targetNode.content);
const pathQuery = pathQueryFor(nodePath);
const fullQuery = fullTextQueryFor(targetNode.content);

run("open", [url]);
assertSnapshotIncludes(contentProbe);
run("open", [`${url}?view=raw`]);
assertSnapshotIncludes(contentProbe);
run("open", [`${searchUrl}?q=${encodeURIComponent(pathQuery)}&kind=path`]);
assertSnapshotIncludes("path_substring");
run("open", [`${searchUrl}?q=${encodeURIComponent(fullQuery)}&kind=full`]);
assertSnapshotIncludes("content_fts");
assertSnapshotIncludes("Full text");
run("open", [`${url}?tab=recent`]);
assertSnapshotIncludes("Recent");
run("open", [`${url}?tab=lint`]);
assertSnapshotIncludes("Lint Hints");

console.log(`Wiki browser smoke OK: ${canisterId} ${nodePath}`);

function readUrl() {
  const argIndex = process.argv.indexOf("--url");
  const value = argIndex >= 0 ? process.argv[argIndex + 1] : process.env.WIKI_BROWSER_SMOKE_URL;
  if (!value) {
    throw new Error("missing --url or WIKI_BROWSER_SMOKE_URL");
  }
  return value;
}

function assertSnapshotIncludes(text) {
  let lastOutput = "";
  const deadline = Date.now() + smokeWaitMs;
  while (Date.now() < deadline) {
    lastOutput = snapshotText();
    if (lastOutput.includes(text)) {
      return;
    }
    sleep(pollMs);
  }
  throw new Error(`snapshot missing ${text}\n${lastOutput}`);
}

async function readTargetNode() {
  const response = await fetch(`${targetUrl.origin}/api/wiki/${encodeURIComponent(canisterId)}/node?path=${encodeURIComponent(nodePath)}`);
  if (!response.ok) {
    throw new Error(`smoke target node is not readable: ${response.status} ${await response.text()}`);
  }
  const node = await response.json();
  if (node.kind !== "file" || typeof node.content !== "string" || !node.content.trim()) {
    throw new Error(`smoke target must be an existing file with content: ${nodePath}`);
  }
  return node;
}

function contentProbeFor(content) {
  return content
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => !line.startsWith("- ["))
    .map((line) => line.replace(/^#+\s*/, "").replace(/^-\s*/, ""))
    .find((line) => line.length >= 12) ?? content.trim().slice(0, 40);
}

function pathQueryFor(path) {
  const name = path.split("/").filter(Boolean).at(-1) ?? "index.md";
  return name.replace(/\.[^.]+$/, "") || name;
}

function fullTextQueryFor(content) {
  const ignored = new Set(["facts", "related", "index", "events", "plans", "provenance", "summary", "extracted"]);
  const words = content.match(/[A-Za-z][A-Za-z0-9-]{4,}/g) ?? [];
  const candidate = words.find((word) => !ignored.has(word.toLowerCase()));
  if (!candidate) {
    throw new Error("smoke target content does not contain a usable full-text query term");
  }
  return candidate;
}

function snapshotText() {
  const output = run("snapshot", []);
  const path = output.match(/\[Snapshot\]\(([^)]+)\)/)?.[1];
  if (!path) {
    return output;
  }
  return `${output}\n${readFileSync(path, "utf8")}`;
}

function run(command, args) {
  const result = spawnSync("playwright-cli", [command, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"]
  });
  const output = `${result.stdout}${result.stderr}`;
  if (result.status !== 0) {
    throw new Error(output);
  }
  return output;
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}
