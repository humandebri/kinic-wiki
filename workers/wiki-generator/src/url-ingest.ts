// Where: workers/wiki-generator/src/url-ingest.ts
// What: URL ingest request parsing, source persistence, and request state writes.
// Why: Browser-submitted URLs should become raw sources before wiki draft generation.
import { enqueueSourceJob, loadJob } from "./jobs.js";
import { parseFrontmatter, renderFrontmatter } from "./frontmatter.js";
import { fetchUrlSource, type FetchedUrlSource } from "./url-fetch.js";
import type { RuntimeEnv } from "./env.js";
import type { UrlIngestRequest, WikiNode, WorkerConfig } from "./types.js";
import type { VfsClient } from "./vfs.js";

export function parseUrlIngestRequest(node: WikiNode): UrlIngestRequest | null {
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
    error: document.fields.error
  };
}

export function shouldProcessIngestRequest(request: UrlIngestRequest): boolean {
  return request.status === "queued" || request.status === "fetching" || request.status === "source_written";
}

export async function processUrlIngestRequest(env: RuntimeEnv, vfs: VfsClient, config: WorkerConfig, databaseId: string, request: UrlIngestRequest): Promise<void> {
  let current = request;
  try {
    if (current.status !== "source_written") {
      current = await writeRequestState(vfs, databaseId, current, { status: "fetching", error: null });
      const fetched = await fetchUrlSource(current.url, config.maxFetchedBytes);
      const sourcePath = await sourcePathForUrl(config.sourcePrefix, fetched.finalUrl);
      const source = await writeFetchedSource(vfs, databaseId, sourcePath, current.path, fetched);
      current = await writeRequestState(vfs, databaseId, current, { status: "source_written", sourcePath: source.path, error: null });
    }
    if (!current.sourcePath) throw new Error("source_path is missing after source write");
    const source = await requireSource(vfs, databaseId, current.sourcePath);
    const queued = await enqueueSourceJob(env, {
      kind: "source",
      databaseId,
      sourcePath: source.path,
      sourceEtag: source.etag,
      requestPath: current.path
    });
    if (!queued) {
      const job = await loadJob(env.DB, databaseId, source.path);
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

async function writeFetchedSource(vfs: VfsClient, databaseId: string, path: string, requestPath: string, fetched: FetchedUrlSource): Promise<WikiNode> {
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
      captured_at: capturedAt,
      request_path: requestPath
    },
    [`# ${title}`, "", `Source URL: ${fetched.finalUrl}`, "", fetched.text].join("\n")
  );
  await vfs.writeNode({
    databaseId,
    path,
    kind: "source",
    content,
    metadataJson: JSON.stringify({ source_type: "url", url: fetched.url, final_url: fetched.finalUrl, request_path: requestPath }),
    expectedEtag: existing?.etag ?? null
  });
  return requireSource(vfs, databaseId, path);
}

async function writeRequestState(
  vfs: VfsClient,
  databaseId: string,
  request: UrlIngestRequest,
  updates: { status: UrlIngestRequest["status"]; sourcePath?: string | null; targetPath?: string | null; error?: string | null }
): Promise<UrlIngestRequest> {
  const fields = {
    kind: "kinic.url_ingest_request",
    schema_version: "1",
    status: updates.status,
    url: request.url,
    requested_by: request.requestedBy,
    requested_at: request.requestedAt,
    source_path: updates.sourcePath === undefined ? request.sourcePath : updates.sourcePath,
    target_path: updates.targetPath === undefined ? request.targetPath : updates.targetPath,
    error: updates.error === undefined ? request.error : updates.error
  };
  await vfs.writeNode({
    databaseId,
    path: request.path,
    kind: "source",
    content: renderFrontmatter(fields, "# URL Ingest Request\n"),
    metadataJson: "{}",
    expectedEtag: request.etag
  });
  const updated = await vfs.readNode(databaseId, request.path);
  if (!updated) throw new Error(`request disappeared: ${request.path}`);
  const parsed = parseUrlIngestRequest(updated);
  if (!parsed) throw new Error(`request became invalid: ${request.path}`);
  return parsed;
}

async function requireSource(vfs: VfsClient, databaseId: string, path: string): Promise<WikiNode> {
  const source = await vfs.readNode(databaseId, path);
  if (!source) throw new Error(`source node not found: ${path}`);
  if (source.kind !== "source") throw new Error(`node is not a source: ${path}`);
  return source;
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message.slice(0, 4000) : String(error).slice(0, 4000);
}
