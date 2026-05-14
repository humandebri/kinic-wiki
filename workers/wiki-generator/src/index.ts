// Where: workers/wiki-generator/src/index.ts
// What: Cloudflare Worker entrypoints for manual, cron, and queue triggers.
// Why: Generation should run outside the wiki browser UI server.
import { isAuthorized } from "./auth.js";
import { parseManualRunInput, parseQueueMessage, processQueueMessage, runManual } from "./processing.js";
import { scanSources } from "./scheduler.js";
import { parseUrlIngestTriggerInput, triggerUrlIngestRequest } from "./url-ingest.js";
import type { QueueMessage } from "./types.js";
import type { RuntimeEnv } from "./env.js";

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    if (request.method === "OPTIONS" && url.pathname === "/url-ingest") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }
    if (request.method === "POST" && url.pathname === "/url-ingest") {
      let body: unknown;
      try {
        body = await request.json();
      } catch {
        return jsonResponse({ error: "invalid JSON body" }, 400, CORS_HEADERS);
      }
      const input = parseUrlIngestTriggerInput(body);
      if (typeof input === "string") {
        return jsonResponse({ error: input }, 400, CORS_HEADERS);
      }
      ctx.waitUntil(
        triggerUrlIngestRequest(env, input).catch((error) => {
          console.error("url ingest trigger failed", errorMessage(error));
        })
      );
      return jsonResponse({ accepted: true, databaseId: input.databaseId, requestPath: input.requestPath }, 202, CORS_HEADERS);
    }
    if (request.method !== "POST" || url.pathname !== "/run") {
      return jsonResponse({ error: "not found" }, 404);
    }
    if (!env.KINIC_WIKI_WORKER_TOKEN) {
      return jsonResponse({ error: "KINIC_WIKI_WORKER_TOKEN is required" }, 503);
    }
    if (!(await isAuthorized(request, env.KINIC_WIKI_WORKER_TOKEN))) {
      return jsonResponse({ error: "unauthorized" }, 401);
    }
    let body: unknown;
    try {
      body = await request.json();
    } catch {
      return jsonResponse({ error: "invalid JSON body" }, 400);
    }
    const input = parseManualRunInput(body);
    if (typeof input === "string") {
      return jsonResponse({ error: input }, 400);
    }
    try {
      return await runManual(env, input);
    } catch (error) {
      return jsonResponse({ error: errorMessage(error) }, 500);
    }
  },

  scheduled(_controller, env, ctx): void {
    ctx.waitUntil(scanSources(env));
  },

  async queue(batch, env): Promise<void> {
    for (const message of batch.messages) {
      const parsed = parseQueueMessage(message.body);
      if (!parsed) {
        message.ack();
        continue;
      }
      await processQueueMessage(env, parsed);
      message.ack();
    }
  }
} satisfies ExportedHandler<RuntimeEnv, QueueMessage>;

const CORS_HEADERS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "POST, OPTIONS",
  "access-control-allow-headers": "content-type"
};

function jsonResponse(body: unknown, status: number, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", ...headers }
  });
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
