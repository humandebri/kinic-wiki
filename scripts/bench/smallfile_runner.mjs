// Where: scripts/bench/smallfile_runner.mjs
// What: Execute fixed metadata scenarios and persist raw JSON per scenario.
// Why: Public VFS review needs comparable metadata numbers, temperature labels, and sync-per-op diagnostics.
import fs from "fs";
import path from "path";
import { spawn } from "child_process";
import {
  argMap,
  chunkIndices,
  crossDirRenamedPath,
  elapsedSeconds,
  filePath,
  fsyncPath,
  mkdirRmdirPath,
  nowNs,
  prepareWorkspace,
  sameDirRenamedPath,
  walkRecursive
} from "./smallfile_common.mjs";

const [, , mode, ...args] = process.argv;

function runWorker(workerArgs) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [new URL(import.meta.url).pathname, "worker", ...workerArgs], {
      stdio: ["ignore", "pipe", "pipe"]
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", chunk => { stdout += chunk.toString(); });
    child.stderr.on("data", chunk => { stderr += chunk.toString(); });
    child.on("error", reject);
    child.on("close", code => {
      if (code !== 0) {
        reject(new Error(stderr.trim() || `worker exited with code ${code}`));
        return;
      }
      resolve(JSON.parse(stdout));
    });
  });
}

async function executeOperation(params) {
  prepareWorkspace(params);
  const start = nowNs();
  const results = await Promise.all(
    Array.from({ length: params.clients }, (_, workerIndex) =>
      runWorker([
        "--run-dir", params.runDir,
        "--file-count", String(params.fileCount),
        "--file-size", String(params.fileSize),
        "--dir-width", String(params.dirWidth),
        "--clients", String(params.clients),
        "--worker-index", String(workerIndex),
        "--operation", params.operation,
        "--payload", params.payload,
        "--append-payload", params.appendPayload
      ])
    )
  );
  const totalSeconds = elapsedSeconds(start);
  const operationCount = results.reduce((sum, item) => sum + item.operation_count, 0);
  const syncCount = results.reduce((sum, item) => sum + item.sync_count, 0);
  return {
    operation: params.operation,
    total_seconds: totalSeconds,
    ops_per_sec: operationCount / totalSeconds,
    operation_count: operationCount,
    sync_count: syncCount,
    ops_unit: results[0]?.ops_unit ?? "ops"
  };
}

function runOperationSubprocess(parentParams, operation) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [
      new URL(import.meta.url).pathname,
      "op",
      "--run-dir", path.join(parentParams.runDir, operation),
      "--file-count", String(parentParams.fileCount),
      "--file-size", String(parentParams.fileSize),
      "--dir-width", String(parentParams.dirWidth),
      "--clients", String(parentParams.clients),
      "--operation", operation,
      "--payload", parentParams.payload,
      "--append-payload", parentParams.appendPayload
    ], { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", chunk => { stdout += chunk.toString(); });
    child.stderr.on("data", chunk => { stderr += chunk.toString(); });
    child.on("error", reject);
    child.on("close", code => {
      if (code !== 0) {
        reject(new Error(stderr.trim() || `op exited with code ${code}`));
        return;
      }
      resolve(JSON.parse(stdout));
    });
  });
}

async function runParent() {
  const params = argMap(args);
  const scenario = params.get("--scenario");
  const runDir = params.get("--run-dir");
  const rawJson = params.get("--raw-json");
  const parentParams = {
    scenario,
    runDir,
    fileCount: Number(params.get("--file-count")),
    fileSize: Number(params.get("--file-size")),
    dirWidth: Number(params.get("--dir-width")),
    clients: Number(params.get("--clients")),
    temperature: params.get("--temperature"),
    syncPolicy: params.get("--sync-policy"),
    directoryShape: params.get("--directory-shape"),
    payload: "a".repeat(Number(params.get("--file-size"))),
    appendPayload: "b".repeat(128)
  };
  const operations = params.get("--operations").split(",");

  fs.rmSync(runDir, { recursive: true, force: true });
  fs.mkdirSync(runDir, { recursive: true });

  const results = [];
  for (const operation of operations) {
    if (parentParams.temperature === "cold_process_restart") {
      results.push(await runOperationSubprocess(parentParams, operation));
    } else {
      results.push(await executeOperation({ ...parentParams, operation, runDir: path.join(runDir, operation) }));
    }
  }

  fs.writeFileSync(rawJson, JSON.stringify({
    scenario,
    temperature: parentParams.temperature,
    sync_policy: parentParams.syncPolicy,
    file_count: parentParams.fileCount,
    file_size_bytes: parentParams.fileSize,
    dir_width: parentParams.dirWidth,
    directory_shape: parentParams.directoryShape,
    concurrent_clients: parentParams.clients,
    operations: results
  }, null, 2));
}

async function runOpMode() {
  const params = argMap(args);
  const result = await executeOperation({
    runDir: params.get("--run-dir"),
    fileCount: Number(params.get("--file-count")),
    fileSize: Number(params.get("--file-size")),
    dirWidth: Number(params.get("--dir-width")),
    clients: Number(params.get("--clients")),
    operation: params.get("--operation"),
    payload: params.get("--payload"),
    appendPayload: params.get("--append-payload")
  });
  process.stdout.write(JSON.stringify(result));
}

function runWorkerMode() {
  const params = argMap(args);
  const runDir = params.get("--run-dir");
  const fileCount = Number(params.get("--file-count"));
  const dirWidth = Number(params.get("--dir-width"));
  const clients = Number(params.get("--clients"));
  const workerIndex = Number(params.get("--worker-index"));
  const operation = params.get("--operation");
  const payload = params.get("--payload");
  const appendPayload = params.get("--append-payload");
  const [startIndex, endIndex] = chunkIndices(fileCount, clients, workerIndex);
  const result = { operation_count: 0, ops_unit: "files", sync_count: 0 };

  for (let index = startIndex; index < endIndex; index += 1) {
    if (operation === "create" || operation === "create_sync_each") {
      const target = filePath(runDir, dirWidth, index);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, payload);
      if (operation === "create_sync_each") {
        fsyncPath(target);
        fsyncPath(path.dirname(target));
        result.sync_count += 2;
      }
    } else if (operation === "small_append" || operation === "small_append_sync_each") {
      const target = filePath(runDir, dirWidth, index);
      fs.appendFileSync(target, appendPayload);
      if (operation === "small_append_sync_each") {
        fsyncPath(target);
        result.sync_count += 1;
      }
    } else if (operation === "stat") {
      fs.statSync(filePath(runDir, dirWidth, index));
    } else if (operation === "open_close") {
      const fd = fs.openSync(filePath(runDir, dirWidth, index), "r");
      fs.closeSync(fd);
    } else if (operation === "rename_same_dir") {
      fs.renameSync(filePath(runDir, dirWidth, index), sameDirRenamedPath(runDir, dirWidth, index));
    } else if (operation === "rename_cross_dir" || operation === "rename_cross_dir_sync_each") {
      const source = filePath(runDir, dirWidth, index);
      const target = crossDirRenamedPath(runDir, dirWidth, fileCount, index);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.renameSync(source, target);
      if (operation === "rename_cross_dir_sync_each") {
        fsyncPath(path.dirname(source));
        fsyncPath(path.dirname(target));
        result.sync_count += 2;
      }
    } else if (operation === "unlink" || operation === "unlink_sync_each") {
      const target = filePath(runDir, dirWidth, index);
      fs.unlinkSync(target);
      if (operation === "unlink_sync_each") {
        fsyncPath(path.dirname(target));
        result.sync_count += 1;
      }
    } else if (operation === "mkdir_rmdir") {
      const target = mkdirRmdirPath(runDir, dirWidth, index);
      fs.mkdirSync(target, { recursive: true });
      fs.rmdirSync(target);
    }
    result.operation_count += 1;
  }

  if (operation === "readdir_single") {
    result.operation_count = 0;
    result.ops_unit = "directories";
    const startDir = Math.floor(startIndex / dirWidth);
    const endDir = Math.ceil(endIndex / dirWidth);
    for (let dirIndex = startDir; dirIndex < endDir; dirIndex += 1) {
      fs.readdirSync(path.join(runDir, `dir-${dirIndex}`), { withFileTypes: true });
      result.operation_count += 1;
    }
  } else if (operation === "readdir_recursive") {
    result.operation_count = workerIndex === 0 ? walkRecursive(runDir) : 0;
    result.ops_unit = "files";
  }

  process.stdout.write(JSON.stringify(result));
}

if (mode === "worker") {
  runWorkerMode();
} else if (mode === "op") {
  await runOpMode();
} else {
  await runParent();
}
