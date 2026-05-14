// Where: workers/wiki-generator/src/index.ts
// What: Cloudflare Worker entrypoints for manual, URL ingest, and queue triggers.
// Why: Generation should run outside the wiki browser UI server.
import { isAuthorized } from "./auth.js";
import { parseManualRunInput, parseQueueMessage, processQueueMessage, runManual } from "./processing.js";
import { parseUrlIngestTriggerInput, prepareUrlIngestTrigger, triggerUrlIngestRequest, UrlIngestTriggerError } from "./url-ingest.js";
import type { QueueMessage } from "./types.js";
import type { RuntimeEnv } from "./env.js";

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    if (request.method === "POST" && url.pathname === "/url-ingest") {
      const authError = await workerAuthError(request, env);
      if (authError) return authError;
      let body: unknown;
      try {
        body = await request.json();
      } catch {
        return jsonResponse({ error: "invalid JSON body" }, 400);
      }
      const input = parseUrlIngestTriggerInput(body);
      if (typeof input === "string") {
        return jsonResponse({ error: input }, 400);
      }
      let triggerContext: Awaited<ReturnType<typeof prepareUrlIngestTrigger>>;
      try {
        triggerContext = await prepareUrlIngestTrigger(env, input);
      } catch (error) {
        const status = error instanceof UrlIngestTriggerError ? error.status : 500;
        return jsonResponse({ error: errorMessage(error) }, status);
      }
      ctx.waitUntil(
        triggerUrlIngestRequest(env, input, triggerContext).catch((error) => {
          console.error("url ingest trigger failed", errorMessage(error));
        })
      );
      return jsonResponse({ accepted: true, databaseId: input.databaseId, requestPath: input.requestPath }, 202);
    }
    if (request.method !== "POST" || url.pathname !== "/run") {
      return jsonResponse({ error: "not found" }, 404);
    }
    const authError = await workerAuthError(request, env);
    if (authError) return authError;
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

function jsonResponse(body: unknown, status: number, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", ...headers }
  });
}

async function workerAuthError(request: Request, env: RuntimeEnv): Promise<Response | null> {
  if (!env.KINIC_WIKI_WORKER_TOKEN) {
    return jsonResponse({ error: "KINIC_WIKI_WORKER_TOKEN is required" }, 503);
  }
  if (!(await isAuthorized(request, env.KINIC_WIKI_WORKER_TOKEN))) {
    return jsonResponse({ error: "unauthorized" }, 401);
  }
  return null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
