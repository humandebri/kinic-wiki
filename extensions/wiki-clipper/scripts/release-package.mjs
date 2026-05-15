// Where: extensions/wiki-clipper/scripts/release-package.mjs
// What: Create the Chrome Web Store upload zip from built extension files.
// Why: The store package must contain runtime files only, not source, tests, env files, or dependencies.
import { mkdir, readdir, rm, stat } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import manifest from "../manifest.json" with { type: "json" };

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const releaseDir = resolve(root, "release");
const zipPath = resolve(releaseDir, `kinic-wiki-clipper-${manifest.version}.zip`);
const packageRoots = ["manifest.json", "icons", "offscreen", "popup", "dist"];

await rm(releaseDir, { recursive: true, force: true });
await mkdir(releaseDir, { recursive: true });

const files = [];
for (const packageRoot of packageRoots) {
  await collectPackageFiles(packageRoot, files);
}

const result = spawnSync("/usr/bin/zip", ["-q", "-X", zipPath, ...files], {
  cwd: root,
  encoding: "utf8"
});
if (result.status !== 0) {
  console.error(result.stderr || result.stdout || "zip failed");
  process.exit(result.status || 1);
}

console.log(`created ${zipPath}`);

async function collectPackageFiles(relativePath, files) {
  const absolutePath = resolve(root, relativePath);
  const info = await stat(absolutePath);
  if (info.isDirectory()) {
    for (const entry of await readdir(absolutePath)) {
      if (entry.startsWith(".")) continue;
      await collectPackageFiles(`${relativePath}/${entry}`, files);
    }
    return;
  }
  files.push(relativePath);
}
