import { NextRequest, NextResponse } from "next/server";
import {
  clampLimit,
  handleApiError,
  invalidCanisterResponse,
  missingParam,
  type RouteParams
} from "@/lib/api";
import { searchNodePaths } from "@/lib/vfs-client";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(request: NextRequest, context: { params: RouteParams }) {
  const { canisterId } = await context.params;
  const invalid = invalidCanisterResponse(canisterId);
  if (invalid) {
    return invalid;
  }
  const query = request.nextUrl.searchParams.get("q");
  if (!query) {
    return missingParam("q");
  }
  const limit = clampLimit(request.nextUrl.searchParams.get("limit"), 20);
  const prefix = request.nextUrl.searchParams.get("prefix");
  try {
    return NextResponse.json(await searchNodePaths(canisterId, query, limit, prefix));
  } catch (error) {
    return handleApiError(error);
  }
}
