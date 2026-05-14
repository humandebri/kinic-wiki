// Where: workers/wiki-generator/src/url-ingest.ts
// What: URL ingest request parsing, source persistence, and request state writes.
// Why: Browser-submitted URLs should become raw sources before wiki draft generation.
import { enqueueSourceJob, loadJob } from "./jobs.js";
import { loadConfig } from "./config.js";
import { parseFrontmatter, renderFrontmatter } from "./frontmatter.js";
import { fetchUrlSource, type FetchedUrlSource } from "./url-fetch.js";
import type { RuntimeEnv } from "./env.js";
import type { UrlIngestRequest, UrlIngestTriggerInput, WikiNode, WorkerConfig, WriteNodeAck } from "./types.js";
import { createVfsClient, type VfsClient } from "./vfs.js";

export function parseUrlIngestTriggerInput(value: unknown): UrlIngestTriggerInput | string {
  if (!isObject(value)) return "body must include databaseId and requestPath";
  const databaseId = value.databaseId;
  const requestPath = value.requestPath;
  if (typeof databaseId !== "string" || databaseId.length === 0) return "databaseId is required";
  if (typeof requestPath !== "string" || requestPath.length === 0) return "requestPath is required";
  return { databaseId, requestPath };
}

export async function triggerUrlIngestRequest(env: RuntimeEnv, input: UrlIngestTriggerInput): Promise<void> {
  const config = loadConfig(env);
  validateIngestRequestPath(input.requestPath, config.ingestRequestPrefix);
  const vfs = await createVfsClient(config, env.KINIC_WIKI_WORKER_IDENTITY_PEM);
  const node = await vfs.readNode(input.databaseId, input.requestPath);
  if (!node) throw new Error(`ingest request not found: ${input.requestPath}`);
  const request = parseUrlIngestRequest(node);
  if (!request) throw new Error(`invalid ingest request: ${input.requestPath}`);
  if (!shouldProcessIngestRequest(request)) return;
  await processUrlIngestRequest(env, vfs, config, input.databaseId, request);
}

export function parseUrlIngestRequest(node: WikiNode): UrlIngestRequest | null {
  if (node.kind !== "file") return null;
  const document = parseFrontmatter(node.content);
  if (!document) return null;
  if (document.fields.kind !== "kinic.url_ingest_request") return null;
  if (document.fields.schema_version !== "1") return null;
  const status = ingestStatus(document.fields.status);
  const url = document.fields.url;
  if (!status || !url) return null;
  return {
    path: node.path,
    etag: node.etag,
    status,
    url,
    requestedBy: document.fields.requested_by ?? "",
    requestedAt: document.fields.requested_at ?? "",
    sourcePath: document.fields.source_path,
    targetPath: document.fields.target_path,
    finishedAt: document.fields.finished_at ?? null,
    error: document.fields.error
  };
}

export function shouldProcessIngestRequest(request: UrlIngestRequest): boolean {
  return request.status === "queued" || request.status === "fetching" || request.status === "source_written";
}

export async function processUrlIngestRequest(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string, request: UrlIngestRequest): Promise<void> {
  let current = request;
  try {
    let sourceAck: WriteNodeAck | null = null;
    if (current.status !== "source_written") {
      current = await writeRequestState(vfs, databaseId, current, { status: "fetching", error: null });
      const fetched = await fetchUrlSource(current.url, config.maxFetchedBytes);
      const sourcePath = await sourcePathForUrl(config.sourcePrefix, fetched.finalUrl);
      sourceAck = await writeFetchedSource(vfs, databaseId, sourcePath, current.path, fetched);
      current = await writeRequestState(vfs, databaseId, current, { status: "source_written", sourcePath: sourceAck.path, error: null });
    }
    if (!current.sourcePath) throw new Error("source_path is missing after source write");
    sourceAck = sourceAck ?? (await requireSourceAck(vfs, databaseId, current.sourcePath));
    const queued = await enqueueSourceJob(env, {
      kind: "source",
      databaseId,
      sourcePath: sourceAck.path,
      sourceEtag: sourceAck.etag,
      requestPath: current.path
    });
    if (!queued) {
      const job = await loadJob(env.DB, databaseId, sourceAck.path);
      if (job?.status === "completed") {
        await writeRequestState(vfs, databaseId, current, { status: "completed", targetPath: job.target_path, error: null });
        return;
      }
    }
    await writeRequestState(vfs, databaseId, current, { status: "generating", error: null });
  } catch (error) {
    await writeRequestState(vfs, databaseId, current, { status: "failed", error: errorMessage(error) });
  }
}

export async function markIngestRequestCompleted(vfs: VfsClient, databaseId: string, requestPath: string, sourcePath: string, targetPath: string): Promise<void> {
  const node = await vfs.readNode(databaseId, requestPath);
  if (!node) return;
  const request = parseUrlIngestRequest(node);
  if (!request) return;
  await writeRequestState(vfs, databaseId, request, { status: "completed", sourcePath, targetPath, error: null });
}

export async function markIngestRequestFailed(vfs: VfsClient, databaseId: string, requestPath: string, error: string): Promise<void> {
  const node = await vfs.readNode(databaseId, requestPath);
  if (!node) return;
  const request = parseUrlIngestRequest(node);
  if (!request) return;
  await writeRequestState(vfs, databaseId, request, { status: "failed", error });
}

async function writeFetchedSource(vfs: VfsClient, databaseId: string, path: string, _requestPath: string, fetched: FetchedUrlSource): Promise<WriteNodeAck> {
  const existing = await vfs.readNode(databaseId, path);
  const capturedAt = new Date().toISOString();
  const title = fetched.title ?? fetched.finalUrl;
  const content = renderFrontmatter(
    {
      kind: "kinic.raw_web_source",
      schema_version: "1",
      url: fetched.url,
      final_url: fetched.finalUrl,
      title,
      content_type: fetched.contentType,
      captured_at: capturedAt
    },
    [`# ${title}`, "", `Source URL: ${fetched.finalUrl}`, "", fetched.text].join("\n")
  );
  const ack = await vfs.writeNode({
    databaseId,
    path,
    kind: "source",
    content,
    metadataJson: JSON.stringify({ source_type: "url", url: fetched.url, final_url: fetched.finalUrl }),
    expectedEtag: existing?.etag ?? null
  });
  if (ack.kind !== "source") throw new Error(`write_node returned non-source kind: ${ack.path}`);
  return ack;
}

async function writeRequestState(
  vfs: VfsClient,
  databaseId: string,
  request: UrlIngestRequest,
  updates: { status: UrlIngestRequest["status"]; sourcePath?: string | null; targetPath?: string | null; error?: string | null }
): Promise<UrlIngestRequest> {
  const finishedAt = isTerminalStatus(updates.status) ? (request.finishedAt ?? new Date().toISOString()) : request.finishedAt;
  const fields = {
    kind: "kinic.url_ingest_request",
    schema_version: "1",
    status: updates.status,
    url: request.url,
    requested_by: request.requestedBy,
    requested_at: request.requestedAt,
    source_path: updates.sourcePath === undefined ? request.sourcePath : updates.sourcePath,
    target_path: updates.targetPath === undefined ? request.targetPath : updates.targetPath,
    finished_at: finishedAt,
    error: updates.error === undefined ? request.error : updates.error
  };
  const ack = await vfs.writeNode({
    databaseId,
    path: request.path,
    kind: "file",
    content: renderFrontmatter(fields, "# URL Ingest Request\n"),
    metadataJson: "{}",
    expectedEtag: request.etag
  });
  if (ack.kind !== "file") throw new Error(`write_node returned non-file kind: ${ack.path}`);
  return {
    path: request.path,
    etag: ack.etag,
    status: updates.status,
    url: request.url,
    requestedBy: request.requestedBy,
    requestedAt: request.requestedAt,
    sourcePath: fields.source_path,
    targetPath: fields.target_path,
    finishedAt: fields.finished_at,
    error: fields.error
  };
}

async function requireSourceAck(vfs: VfsClient, databaseId: string, path: string): Promise<WriteNodeAck> {
  const source = await vfs.readNode(databaseId, path);
  if (!source) throw new Error(`source node not found: ${path}`);
  if (source.kind !== "source") throw new Error(`node is not a source: ${path}`);
  return { path: source.path, kind: source.kind, etag: source.etag };
}

async function sourcePathForUrl(sourcePrefix: string, finalUrl: string): Promise<string> {
  const id = `web-${(await sha256Hex(finalUrl)).slice(0, 16)}`;
  return `${sourcePrefix}/${id}/${id}.md`;
}

async function sha256Hex(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function ingestStatus(value: string | null | undefined): UrlIngestRequest["status"] | null {
  if (value === "queued" || value === "fetching" || value === "source_written" || value === "generating" || value === "completed" || value === "failed") {
    return value;
  }
  return null;
}

function isTerminalStatus(status: UrlIngestRequest["status"]): boolean {
  return status === "completed" || status === "failed";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message.slice(0, 4000) : String(error).slice(0, 4000);
}

function validateIngestRequestPath(path: string, prefix: string): void {
  if (!path.startsWith(`${prefix}/`) || !path.endsWith(".md")) {
    throw new Error(`non-canonical ingest request path: ${path}`);
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
