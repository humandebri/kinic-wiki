// Where: wikibrowser/app/api/sources/extract/route.ts
// What: Fetch source pages for client-side text extraction.
// Why: Browsers cannot reliably fetch arbitrary source pages because of CORS.

const FETCH_TIMEOUT_MS = 12_000;
const MAX_HTML_BYTES = 2_000_000;
const MAX_REDIRECTS = 5;

export async function POST(request: Request): Promise<Response> {
  let url: URL;
  try {
    const body: unknown = await request.json();
    if (!isRecord(body) || typeof body.url !== "string") {
      return jsonError("url is required", 400);
    }
    url = parseAllowedUrl(body.url.trim());
  } catch {
    return jsonError("invalid request", 400);
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const { response, finalUrl } = await fetchAllowedSourceUrl(url, controller.signal);
    if (!response.ok) {
      return jsonError(`fetch failed: ${response.status}`, 502);
    }
    const contentType = response.headers.get("content-type") ?? "";
    if (contentType && !contentType.toLowerCase().includes("html")) {
      return jsonError("response is not html", 415);
    }
    const html = await boundedText(response);
    return Response.json({ url: finalUrl.toString(), html });
  } catch (error) {
    const message = error instanceof Error && error.name === "AbortError" ? "fetch timed out" : "fetch failed";
    return jsonError(message, 502);
  } finally {
    clearTimeout(timeout);
  }
}

function parseAllowedUrl(value: string): URL {
  const url = new URL(value);
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("url must use http or https");
  }
  if (isBlockedHostname(url.hostname)) {
    throw new Error("url hostname is not allowed");
  }
  url.hash = "";
  return url;
}

async function fetchAllowedSourceUrl(firstUrl: URL, signal: AbortSignal): Promise<{ response: Response; finalUrl: URL }> {
  let currentUrl = firstUrl;
  for (let redirectCount = 0; redirectCount <= MAX_REDIRECTS; redirectCount += 1) {
    const response = await fetch(currentUrl.toString(), {
      headers: {
        accept: "text/html,application/xhtml+xml",
        "user-agent": "KinicWikiSourceClip/1.0"
      },
      redirect: "manual",
      signal
    });
    if (!isRedirectStatus(response.status)) {
      return { response, finalUrl: currentUrl };
    }
    if (redirectCount === MAX_REDIRECTS) {
      throw new Error("too many redirects");
    }
    const location = response.headers.get("location");
    if (!location) {
      throw new Error("redirect missing location");
    }
    currentUrl = parseAllowedUrl(new URL(location, currentUrl.toString()).toString());
  }
  throw new Error("too many redirects");
}

function isRedirectStatus(status: number): boolean {
  return status === 301 || status === 302 || status === 303 || status === 307 || status === 308;
}

async function boundedText(response: Response): Promise<string> {
  const reader = response.body?.getReader();
  if (!reader) {
    const text = await response.text();
    if (new TextEncoder().encode(text).length > MAX_HTML_BYTES) {
      throw new Error("response too large");
    }
    return text;
  }

  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const next = await reader.read();
    if (next.done) {
      break;
    }
    total += next.value.byteLength;
    if (total > MAX_HTML_BYTES) {
      throw new Error("response too large");
    }
    chunks.push(next.value);
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

function jsonError(error: string, status: number): Response {
  return Response.json({ error }, { status });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isBlockedHostname(hostname: string): boolean {
  const normalized = hostname.toLowerCase();
  if (normalized.startsWith("[") || normalized.includes(":")) return true;
  if (normalized === "localhost" || normalized.endsWith(".localhost")) return true;
  if (normalized === "0.0.0.0") return true;
  const ipv4 = normalized.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
  if (!ipv4) return false;
  const first = Number(ipv4[1]);
  const second = Number(ipv4[2]);
  if (first === 10 || first === 127) return true;
  if (first === 169 && second === 254) return true;
  if (first === 172 && second >= 16 && second <= 31) return true;
  return first === 192 && second === 168;
}
