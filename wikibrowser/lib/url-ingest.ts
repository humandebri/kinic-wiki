import type { Identity } from "@icp-sdk/core/agent";
import { writeNodeAuthenticated } from "@/lib/vfs-client";

export type CreatedUrlIngestRequest = {
  requestPath: string;
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
    kind: "source",
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
      "error: null",
      "---",
      "",
      "# URL Ingest Request",
      ""
    ].join("\n"),
    metadataJson: JSON.stringify({ request_type: "url_ingest", url: normalizedUrl }),
    expectedEtag: null
  });
  return { requestPath };
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
