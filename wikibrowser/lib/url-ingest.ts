import type { Identity } from "@icp-sdk/core/agent";
import { writeNodeAuthenticated } from "@/lib/vfs-client";

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
  const trigger = await triggerWorker(databaseId, requestPath);
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

async function triggerWorker(databaseId: string, requestPath: string): Promise<{ ok: boolean; error: string | null }> {
  const baseUrl = process.env.NEXT_PUBLIC_KINIC_WIKI_GENERATOR_URL?.trim();
  if (!baseUrl) return { ok: false, error: "NEXT_PUBLIC_KINIC_WIKI_GENERATOR_URL is not configured" };
  let endpoint: URL;
  try {
    endpoint = new URL("/url-ingest", baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`);
  } catch {
    return { ok: false, error: "NEXT_PUBLIC_KINIC_WIKI_GENERATOR_URL is invalid" };
  }
  try {
    const response = await fetch(endpoint.toString(), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ databaseId, requestPath })
    });
    if (!response.ok) {
      return { ok: false, error: `worker trigger failed: HTTP ${response.status}` };
    }
    return { ok: true, error: null };
  } catch (cause) {
    return { ok: false, error: cause instanceof Error ? cause.message : "worker trigger failed" };
  }
}
