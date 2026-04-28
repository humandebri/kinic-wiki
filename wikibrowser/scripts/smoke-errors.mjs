import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const baseUrl = readBaseUrl();
const canisterId = readCanisterId();
const smokeWaitMs = 30_000;
const pollMs = 500;

run("open", [`${baseUrl}/${encodeURIComponent(canisterId)}/Wiki/does-not-exist.md`]);
assertSnapshotIncludes("No wiki node at this path");
assertSnapshotIncludes("Search this path");

console.log(`Wiki browser error smoke OK: ${canisterId}`);

function readBaseUrl() {
  const argIndex = process.argv.indexOf("--base-url");
  const value = argIndex >= 0 ? process.argv[argIndex + 1] : process.env.WIKI_BROWSER_BASE_URL;
  return (value ?? "http://127.0.0.1:3000").replace(/\/$/, "");
}

function readCanisterId() {
  const argIndex = process.argv.indexOf("--canister-id");
  const value = argIndex >= 0 ? process.argv[argIndex + 1] : process.env.WIKI_BROWSER_CANISTER_ID;
  if (!value) {
    throw new Error("missing --canister-id or WIKI_BROWSER_CANISTER_ID");
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
