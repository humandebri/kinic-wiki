import type { Identity } from "@icp-sdk/core/agent";
import { authorizeUrlIngestTrigger, writeNodeAuthenticated } from "@/lib/vfs-client";

export type CreatedUrlIngestRequest = {
  requestPath: string;
  triggered: boolean;
  triggerError: string | null;
};

export async function createUrlIngestRequest(canisterId: string, databaseId: string, identity: Identity, url: string): Promise<CreatedUrlIngestRequest> {
  const normalizedUrl = normalizedHttpUrl(url);
  const requestId = `${Date.now()}-${crypto.randomUUID()}`;
  const requestPath = `/Sources/ingest-requests/${requestId}.md`;
  const requestedAt = new Date().toISOString();
  const requestedBy = identity.getPrincipal().toText();
  await writeNodeAuthenticated(canisterId, identity, {
    databaseId,
    path: requestPath,
    kind: "file",
    content: [
      "---",
      "kind: kinic.url_ingest_request",
      "schema_version: 1",
      "status: queued",
      `url: ${JSON.stringify(normalizedUrl)}`,
      `requested_by: ${JSON.stringify(requestedBy)}`,
      `requested_at: ${JSON.stringify(requestedAt)}`,
      "source_path: null",
      "target_path: null",
      "finished_at: null",
      "error: null",
      "---",
      "",
      "# URL Ingest Request",
      ""
    ].join("\n"),
    metadataJson: JSON.stringify({ request_type: "url_ingest", url: normalizedUrl }),
    expectedEtag: null
  });
  const nonce = crypto.randomUUID();
  const grant = await authorizeTriggerGrant(canisterId, databaseId, identity, requestPath, nonce);
  if (!grant.ok) {
    return { requestPath, triggered: false, triggerError: grant.error };
  }
  const trigger = await triggerWorker(databaseId, requestPath, nonce);
  return { requestPath, triggered: trigger.ok, triggerError: trigger.error };
}

function normalizedHttpUrl(value: string): string {
  let url: URL;
  try {
    url = new URL(value.trim());
  } catch {
    throw new Error("Enter a valid URL.");
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("URL must use http or https.");
  }
  url.hash = "";
  return url.toString();
}

async function authorizeTriggerGrant(
  canisterId: string,
  databaseId: string,
  identity: Identity,
  requestPath: string,
  nonce: string
): Promise<{ ok: boolean; error: string | null }> {
  try {
    await authorizeUrlIngestTrigger(canisterId, identity, { databaseId, requestPath, nonce });
    return { ok: true, error: null };
  } catch (cause) {
    return { ok: false, error: cause instanceof Error ? cause.message : "worker trigger grant failed" };
  }
}

async function triggerWorker(databaseId: string, requestPath: string, nonce: string): Promise<{ ok: boolean; error: string | null }> {
  try {
    const response = await fetch("/api/url-ingest/trigger", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ databaseId, requestPath, nonce })
    });
    if (!response.ok) {
      return { ok: false, error: `worker trigger failed: HTTP ${response.status}` };
    }
    return { ok: true, error: null };
  } catch (cause) {
    return { ok: false, error: cause instanceof Error ? cause.message : "worker trigger failed" };
  }
}
