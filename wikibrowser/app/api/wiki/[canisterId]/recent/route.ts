import { NextRequest, NextResponse } from "next/server";
import { clampLimit, handleApiError, invalidCanisterResponse, type RouteParams } from "@/lib/api";
import { recentNodes } from "@/lib/vfs-client";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(request: NextRequest, context: { params: RouteParams }) {
  const { canisterId } = await context.params;
  const invalid = invalidCanisterResponse(canisterId);
  if (invalid) {
    return invalid;
  }
  const limit = clampLimit(request.nextUrl.searchParams.get("limit"), 20);
  try {
    return NextResponse.json(await recentNodes(canisterId, limit));
  } catch (error) {
    return handleApiError(error);
  }
}
