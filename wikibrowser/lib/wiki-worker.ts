// Where: wikibrowser/lib/wiki-worker.ts
// What: One-shot raw source to review-ready wiki draft orchestration.
// Why: Server code owns source validation, VFS writes, and conflict checks.
import type { Identity } from "@icp-sdk/core/agent";
import { Ed25519KeyIdentity } from "@icp-sdk/core/identity";
import { generateDraft } from "@/lib/wiki-worker-openai";
import { renderDraftMarkdown, slugForDraft } from "@/lib/wiki-worker-markdown";
import { readNode, searchNodes, writeNodeAuthenticated } from "@/lib/vfs-client";
import type { SearchNodeHit, WikiNode } from "@/lib/types";
import { SCHEMA_VERSION, type WikiWorkerGenerationConfig, type WikiWorkerRunInput, type WikiWorkerRunResult, type WikiDraft } from "@/lib/wiki-worker-types";

const DEFAULT_SOURCE_PREFIX = "/Sources/raw";
const DEFAULT_TARGET_ROOT = "/Wiki/conversations";
const DEFAULT_CONTEXT_PREFIX = "/Wiki";
const DEFAULT_MODEL = "gpt-5.4-mini";
const DEFAULT_REASONING_EFFORT = "medium";
const DEFAULT_MAX_RAW_CHARS = 120_000;
const DEFAULT_CONTEXT_HITS = 8;
const DEFAULT_MAX_OUTPUT_TOKENS = 6_000;

type WorkerConfig = WikiWorkerGenerationConfig & {
  canisterId: string;
  identity: Identity;
  sourcePrefix: string;
  targetRoot: string;
  contextPrefix: string;
  contextHits: number;
};

export type { WikiWorkerRunInput, WikiWorkerRunResult };

export async function runWikiWorkerOnce(input: WikiWorkerRunInput): Promise<WikiWorkerRunResult> {
  const config = loadWorkerConfig(input.canisterId);
  validateSourcePath(input.sourcePath, config.sourcePrefix);

  const source = await readRequiredSource(config.canisterId, input.databaseId, config.identity, input.sourcePath);
  const contextHits = await loadContext(config.canisterId, input.databaseId, source, config);
  const draft = await generateDraft(source, contextHits, config);
  validateDraft(draft, input.sourcePath);

  const targetPath = `${config.targetRoot}/${slugForDraft(draft)}.md`;
  const content = renderDraftMarkdown(draft, source, contextHits);
  if (input.dryRun) {
    return workerResult(input.sourcePath, targetPath, true, false, content, contextHits);
  }

  await writeDraft(config.canisterId, input.databaseId, config.identity, targetPath, content, input.sourcePath);
  await appendWorkerLog(config.canisterId, input.databaseId, config.identity, config.targetRoot, targetPath, input.sourcePath);

  return workerResult(input.sourcePath, targetPath, false, true, content, contextHits);
}

function loadWorkerConfig(canisterIdOverride: string | undefined): WorkerConfig {
  const canisterId = canisterIdOverride ?? process.env.NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID ?? "";
  if (!canisterId) {
    throw new Error("NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID is required");
  }
  return {
    canisterId,
    identity: loadWorkerIdentity(),
    sourcePrefix: process.env.KINIC_WIKI_WORKER_SOURCE_PREFIX ?? DEFAULT_SOURCE_PREFIX,
    targetRoot: process.env.KINIC_WIKI_WORKER_TARGET_ROOT ?? DEFAULT_TARGET_ROOT,
    contextPrefix: process.env.KINIC_WIKI_WORKER_CONTEXT_PREFIX ?? DEFAULT_CONTEXT_PREFIX,
    model: process.env.KINIC_WIKI_WORKER_MODEL ?? DEFAULT_MODEL,
    reasoningEffort: process.env.KINIC_WIKI_WORKER_REASONING_EFFORT ?? DEFAULT_REASONING_EFFORT,
    maxRawChars: numberEnv("KINIC_WIKI_WORKER_MAX_RAW_CHARS", DEFAULT_MAX_RAW_CHARS),
    contextHits: numberEnv("KINIC_WIKI_WORKER_CONTEXT_HITS", DEFAULT_CONTEXT_HITS),
    maxOutputTokens: numberEnv("KINIC_WIKI_WORKER_MAX_OUTPUT_TOKENS", DEFAULT_MAX_OUTPUT_TOKENS)
  };
}

function loadWorkerIdentity(): Identity {
  const json = process.env.KINIC_WIKI_WORKER_IDENTITY_JSON;
  if (!json) {
    throw new Error("KINIC_WIKI_WORKER_IDENTITY_JSON is required");
  }
  return Ed25519KeyIdentity.fromJSON(json);
}

function numberEnv(name: string, fallback: number): number {
  const raw = process.env[name];
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function validateSourcePath(path: string, prefix: string): void {
  const boundary = `${prefix}/`;
  if (!path.startsWith(boundary)) {
    throw new Error(`sourcePath must be under ${prefix}`);
  }
  const parts = path.slice(boundary.length).split("/");
  if (parts.length !== 2 || !parts[0] || parts[1] !== `${parts[0]}.md`) {
    throw new Error(`sourcePath must use ${prefix}/<id>/<id>.md`);
  }
}

async function readRequiredSource(canisterId: string, databaseId: string, identity: Identity, sourcePath: string): Promise<WikiNode> {
  const source = await readNode(canisterId, databaseId, sourcePath, identity);
  if (!source) {
    throw new Error(`source node not found: ${sourcePath}`);
  }
  if (source.kind !== "source") {
    throw new Error(`node is not a source: ${sourcePath}`);
  }
  return source;
}

async function loadContext(canisterId: string, databaseId: string, source: WikiNode, config: WorkerConfig): Promise<SearchNodeHit[]> {
  const query = contextQuery(source.content, source.path);
  if (!query) return [];
  return searchNodes(canisterId, databaseId, query, config.contextHits, config.contextPrefix, config.identity);
}

function contextQuery(content: string, sourcePath: string): string {
  const title = metadataValue(content, "conversation_title") ?? headingTitle(content);
  if (title) return title;
  return sourcePath.split("/").at(-2) ?? "";
}

function metadataValue(content: string, key: string): string | null {
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    const prefix = `- ${key}:`;
    if (trimmed.startsWith(prefix)) {
      const value = trimmed.slice(prefix.length).trim().replace(/^"|"$/g, "");
      return value || null;
    }
  }
  return null;
}

function headingTitle(content: string): string | null {
  const line = content.split("\n").find((item) => item.startsWith("# "));
  return line ? line.slice(2).trim() : null;
}

function validateDraft(draft: WikiDraft, sourcePath: string): void {
  const sections = [draft.key_facts, draft.decisions, draft.open_questions, draft.follow_ups];
  for (const section of sections) {
    for (const item of section) {
      if (item.source_path !== sourcePath) {
        throw new Error(`generated item cites unsupported source: ${item.source_path}`);
      }
    }
  }
}

async function writeDraft(canisterId: string, databaseId: string, identity: Identity, targetPath: string, content: string, sourcePath: string): Promise<void> {
  const existing = await readNode(canisterId, databaseId, targetPath, identity);
  if (existing && !existing.content.includes(sourcePath)) {
    throw new Error(`target exists without matching provenance: ${targetPath}`);
  }
  await writeNodeAuthenticated(canisterId, identity, {
    databaseId,
    path: targetPath,
    kind: "file",
    content,
    metadataJson: JSON.stringify({
      generated_by: "wiki-worker",
      schema_version: SCHEMA_VERSION,
      source_path: sourcePath,
      state: "Draft"
    }),
    expectedEtag: existing?.etag ?? null
  });
}

async function appendWorkerLog(canisterId: string, databaseId: string, identity: Identity, targetRoot: string, targetPath: string, sourcePath: string): Promise<void> {
  const logPath = `${targetRoot}/log.md`;
  const current = await readNode(canisterId, databaseId, logPath, identity);
  const header = "# Conversation Worker Log\n\n";
  const entry = `- ${new Date().toISOString()} generated ${targetPath} from ${sourcePath}`;
  const content = `${current?.content.trimEnd() ?? header.trimEnd()}\n${entry}\n`;
  await writeNodeAuthenticated(canisterId, identity, {
    databaseId,
    path: logPath,
    kind: "file",
    content,
    metadataJson: "{}",
    expectedEtag: current?.etag ?? null
  });
}

function workerResult(sourcePath: string, targetPath: string, dryRun: boolean, wrote: boolean, content: string, contextHits: SearchNodeHit[]): WikiWorkerRunResult {
  return {
    sourcePath,
    targetPath,
    dryRun,
    wrote,
    content,
    contextPaths: contextHits.map((hit) => hit.path)
  };
}
