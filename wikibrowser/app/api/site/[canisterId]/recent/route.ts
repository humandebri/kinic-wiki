import { NextRequest, NextResponse } from "next/server";
import { handleApiError, invalidCanisterResponse, type RouteParams } from "@/lib/api";
import { recentNodes } from "@/lib/vfs-client";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(request: NextRequest, context: { params: RouteParams }) {
  const { canisterId } = await context.params;
  const invalid = invalidCanisterResponse(canisterId);
  if (invalid) {
    return invalid;
  }
  const rawLimit = Number.parseInt(request.nextUrl.searchParams.get("limit") ?? "20", 10);
  const limit = Number.isFinite(rawLimit) ? Math.min(Math.max(rawLimit, 1), 100) : 20;
  try {
    return NextResponse.json(await recentNodes(canisterId, limit));
  } catch (error) {
    return handleApiError(error);
  }
}
