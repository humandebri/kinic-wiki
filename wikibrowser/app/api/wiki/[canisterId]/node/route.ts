import { NextRequest, NextResponse } from "next/server";
import { handleApiError, invalidCanisterResponse, missingParam, type RouteParams } from "@/lib/api";
import { readNode } from "@/lib/vfs-client";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(request: NextRequest, context: { params: RouteParams }) {
  const { canisterId } = await context.params;
  const invalid = invalidCanisterResponse(canisterId);
  if (invalid) {
    return invalid;
  }
  const path = request.nextUrl.searchParams.get("path");
  if (!path) {
    return missingParam("path");
  }
  try {
    const node = await readNode(canisterId, path);
    if (!node) {
      return NextResponse.json({ error: `node not found: ${path}` }, { status: 404 });
    }
    return NextResponse.json(node);
  } catch (error) {
    return handleApiError(error);
  }
}
