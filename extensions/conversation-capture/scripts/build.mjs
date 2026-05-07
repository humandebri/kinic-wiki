// Where: extensions/conversation-capture/scripts/build.mjs
// What: Bundle the MV3 service worker and copy static extension files.
// Why: Chrome cannot resolve npm bare imports from service workers directly.
import { mkdir, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as esbuild from "esbuild";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = resolve(root, "dist");

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });
await esbuild.build({
  entryPoints: {
    "service-worker": resolve(root, "src/service-worker.js"),
    "content-ui": resolve(root, "src/content-ui.tsx")
  },
  outdir: dist,
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "chrome120",
  jsx: "automatic",
  jsxImportSource: "preact",
  legalComments: "none"
});

console.log("built dist/service-worker.js and dist/content-ui.js");
