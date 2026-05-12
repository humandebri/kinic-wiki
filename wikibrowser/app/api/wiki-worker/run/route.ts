// Where: wikibrowser/app/api/wiki-worker/run/route.ts
// What: Codex app server entrypoint for one raw-source wiki draft generation.
// Why: A local server route lets us validate the worker contract before adding queue polling.
import { NextResponse, type NextRequest } from "next/server";
import { runWikiWorkerOnce, type WikiWorkerRunInput } from "@/lib/wiki-worker";

export const runtime = "nodejs";

type RouteBody = {
  canisterId?: string;
  databaseId?: string;
  sourcePath?: string;
  dryRun?: boolean;
};

export async function POST(request: NextRequest): Promise<NextResponse> {
  const authError = authorize(request);
  if (authError) {
    return authError;
  }
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "invalid JSON body" }, { status: 400 });
  }
  const input = parseRouteBody(body);
  if (typeof input === "string") {
    return NextResponse.json({ error: input }, { status: 400 });
  }
  try {
    return NextResponse.json(await runWikiWorkerOnce(input));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}

function authorize(request: NextRequest): NextResponse | null {
  const token = process.env.KINIC_WIKI_WORKER_TOKEN;
  if (!token && process.env.NODE_ENV !== "production") {
    return null;
  }
  if (!token) {
    return NextResponse.json({ error: "KINIC_WIKI_WORKER_TOKEN is required" }, { status: 503 });
  }
  const expected = `Bearer ${token}`;
  if (request.headers.get("authorization") !== expected) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }
  return null;
}

function parseRouteBody(body: unknown): WikiWorkerRunInput | string {
  if (!isRouteBody(body)) {
    return "body must include databaseId and sourcePath";
  }
  if (!body.databaseId) {
    return "databaseId is required";
  }
  if (!body.sourcePath) {
    return "sourcePath is required";
  }
  return {
    canisterId: body.canisterId,
    databaseId: body.databaseId,
    sourcePath: body.sourcePath,
    dryRun: body.dryRun ?? false
  };
}

function isRouteBody(value: unknown): value is RouteBody {
  if (typeof value !== "object" || value === null) return false;
  return (
    (!("canisterId" in value) || value.canisterId === undefined || typeof value.canisterId === "string") &&
    (!("databaseId" in value) || value.databaseId === undefined || typeof value.databaseId === "string") &&
    (!("sourcePath" in value) || value.sourcePath === undefined || typeof value.sourcePath === "string") &&
    (!("dryRun" in value) || value.dryRun === undefined || typeof value.dryRun === "boolean")
  );
}
