import assert from "node:assert/strict";
import { parseSmokeTargetUrl } from "./smoke-ui.mjs";

const canisterId = "t63gs-up777-77776-aaaba-cai";

assert.deepEqual(parseSmokeTargetUrl(`http://localhost:3000/w/${canisterId}/Wiki/space%20name.md`), {
  origin: "http://localhost:3000",
  canisterId,
  nodePath: "/Wiki/space name.md"
});
assert.deepEqual(parseSmokeTargetUrl(`http://localhost:3000/w/${canisterId}/Wiki/%E3%81%82.md`), {
  origin: "http://localhost:3000",
  canisterId,
  nodePath: "/Wiki/あ.md"
});
assert.deepEqual(parseSmokeTargetUrl(`http://localhost:3000/w/${canisterId}/Wiki/100%25.md`), {
  origin: "http://localhost:3000",
  canisterId,
  nodePath: "/Wiki/100%.md"
});
assert.deepEqual(parseSmokeTargetUrl(`http://localhost:3000/w/${canisterId}/Wiki/bad%.md`), {
  origin: "http://localhost:3000",
  canisterId,
  nodePath: "/Wiki/bad%.md"
});

console.log("Smoke URL checks OK");
