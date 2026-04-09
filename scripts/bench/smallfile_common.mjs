// Where: scripts/bench/smallfile_common.mjs
// What: Shared helpers for the smallfile benchmark runner.
// Why: Keeping setup and path logic separate keeps each benchmark file small and reviewable.
import fs from "fs";
import path from "path";

export function argMap(values) {
  const map = new Map();
  for (let i = 0; i < values.length; i += 2) {
    map.set(values[i], values[i + 1]);
  }
  return map;
}

export function nowNs() {
  return process.hrtime.bigint();
}

export function elapsedSeconds(startNs) {
  return Number(process.hrtime.bigint() - startNs) / 1e9;
}

export function chunkIndices(total, chunks, workerIndex) {
  const start = Math.floor((total * workerIndex) / chunks);
  const end = Math.floor((total * (workerIndex + 1)) / chunks);
  return [start, end];
}

function bucketIndex(dirWidth, index) {
  return Math.floor(index / dirWidth);
}

function bucketDir(runDir, dirWidth, index) {
  return path.join(runDir, `dir-${bucketIndex(dirWidth, index)}`);
}

export function filePath(runDir, dirWidth, index) {
  return path.join(bucketDir(runDir, dirWidth, index), `node-${index}.md`);
}

export function sameDirRenamedPath(runDir, dirWidth, index) {
  return path.join(bucketDir(runDir, dirWidth, index), `node-${index}.renamed.md`);
}

export function crossDirRenamedPath(runDir, dirWidth, fileCount, index) {
  const totalDirs = Math.max(1, Math.ceil(fileCount / dirWidth));
  const sourceDir = bucketIndex(dirWidth, index);
  const targetDir = (sourceDir + 1) % (totalDirs + 1);
  return path.join(runDir, `xdir-${targetDir}`, `node-${index}.moved.md`);
}

export function mkdirRmdirPath(runDir, dirWidth, index) {
  return path.join(bucketDir(runDir, dirWidth, index), `dir-node-${index}`);
}

export function walkRecursive(rootDir) {
  const pending = [rootDir];
  let files = 0;
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const nextPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(nextPath);
      } else {
        files += 1;
      }
    }
  }
  return files;
}

export function fsyncPath(targetPath) {
  const fd = fs.openSync(targetPath, "r");
  fs.fsyncSync(fd);
  fs.closeSync(fd);
}

function ensureSeedFiles(runDir, fileCount, dirWidth, payload) {
  for (let index = 0; index < fileCount; index += 1) {
    const nextPath = filePath(runDir, dirWidth, index);
    fs.mkdirSync(path.dirname(nextPath), { recursive: true });
    fs.writeFileSync(nextPath, payload);
  }
}

function ensureCrossDirs(runDir, fileCount, dirWidth) {
  const totalDirs = Math.max(1, Math.ceil(fileCount / dirWidth));
  for (let dirIndex = 0; dirIndex <= totalDirs; dirIndex += 1) {
    fs.mkdirSync(path.join(runDir, `xdir-${dirIndex}`), { recursive: true });
  }
}

export function prepareWorkspace(params) {
  fs.rmSync(params.runDir, { recursive: true, force: true });
  fs.mkdirSync(params.runDir, { recursive: true });

  switch (params.operation) {
    case "create":
    case "create_sync_each":
    case "mkdir_rmdir":
      return;
    case "small_append":
    case "small_append_sync_each":
    case "stat":
    case "open_close":
    case "readdir_single":
    case "readdir_recursive":
    case "unlink":
    case "unlink_sync_each":
    case "rename_same_dir":
    case "rename_cross_dir":
    case "rename_cross_dir_sync_each":
      ensureSeedFiles(params.runDir, params.fileCount, params.dirWidth, params.payload);
      if (params.operation.includes("rename_cross_dir")) {
        ensureCrossDirs(params.runDir, params.fileCount, params.dirWidth);
      }
      return;
    default:
      throw new Error(`unsupported operation: ${params.operation}`);
  }
}
