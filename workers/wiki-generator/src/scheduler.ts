// Where: workers/wiki-generator/src/scheduler.ts
// What: Cron-driven raw source scanning and queue fanout.
// Why: The canister has no push event hook, so v1 uses snapshot/update polling.
import { loadConfig } from "./config.js";
import { enqueueSourceJob, loadCursor, saveCursor } from "./jobs.js";
import { parseUrlIngestRequest, processUrlIngestRequest, shouldProcessIngestRequest } from "./url-ingest.js";
import { createVfsClient, type VfsClient } from "./vfs.js";
import type { WikiNode, WorkerConfig } from "./types.js";
import type { RuntimeEnv } from "./env.js";

export async function scanSources(env: RuntimeEnv): Promise<void> {
  const config = loadConfig(env);
  const vfs = await createVfsClient(config, env.KINIC_WIKI_WORKER_IDENTITY_JSON);
  for (const databaseId of config.databaseIds) {
    await scanIngestRequests(env, vfs, config, databaseId);
    await scanDatabase(env, vfs, config, databaseId);
  }
}

async function scanIngestRequests(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string): Promise<void> {
  let cursor: string | null = null;
  for (;;) {
    const page = await vfs.exportSnapshot(databaseId, config.ingestRequestPrefix, cursor, null);
    for (const node of page.nodes) {
      const request = parseUrlIngestRequest(node);
      if (request && shouldProcessIngestRequest(request)) {
        await processUrlIngestRequest(env, vfs, config, databaseId, request);
      }
    }
    if (!page.nextCursor) break;
    cursor = page.nextCursor;
  }
}

async function scanDatabase(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string): Promise<void> {
  const cursor = await loadCursor(env.DB, databaseId, config.sourcePrefix);
  if (cursor) {
    await scanUpdates(env, vfs, config, databaseId, cursor);
  } else {
    await scanSnapshot(env, vfs, config, databaseId);
  }
}

async function scanSnapshot(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string): Promise<void> {
  let cursor: string | null = null;
  let snapshotRevision: string | null = null;
  for (;;) {
    const page = await vfs.exportSnapshot(databaseId, config.sourcePrefix, cursor, snapshotRevision);
    snapshotRevision = page.snapshotRevision;
    for (const node of page.nodes) {
      await enqueueSourceNode(env, databaseId, node);
    }
    if (!page.nextCursor) break;
    cursor = page.nextCursor;
  }
  if (snapshotRevision) {
    await saveCursor(env.DB, databaseId, config.sourcePrefix, snapshotRevision);
  }
}

async function scanUpdates(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string, knownRevision: string): Promise<void> {
  let cursor: string | null = null;
  let targetRevision: string | null = null;
  for (;;) {
    const page = await vfs.fetchUpdates(databaseId, config.sourcePrefix, knownRevision, cursor, targetRevision);
    targetRevision = page.snapshotRevision;
    for (const node of page.changedNodes) {
      await enqueueSourceNode(env, databaseId, node);
    }
    if (!page.nextCursor) break;
    cursor = page.nextCursor;
  }
  if (targetRevision) {
    await saveCursor(env.DB, databaseId, config.sourcePrefix, targetRevision);
  }
}

async function enqueueSourceNode(env: RuntimeEnv, databaseId: string, node: WikiNode): Promise<void> {
  if (node.kind !== "source") return;
  await enqueueSourceJob(env, {
    databaseId,
    sourcePath: node.path,
    sourceEtag: node.etag
  });
}
