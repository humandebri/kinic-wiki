import { NextResponse } from "next/server";
import { classifyApiError, invalidCanisterIdError } from "@/lib/api-errors";
import { validateCanisterId } from "@/lib/vfs-client";

export type RouteParams = Promise<{ canisterId: string }>;

export function invalidCanisterResponse(canisterId: string): NextResponse | null {
  const result = validateCanisterId(canisterId);
  if (typeof result !== "string") {
    return null;
  }
  return NextResponse.json(invalidCanisterIdError(result), { status: 400 });
}

export function missingParam(name: string): NextResponse {
  return NextResponse.json({ error: `missing ${name}` }, { status: 400 });
}

export function clampLimit(rawValue: string | null, fallback: number): number {
  const rawLimit = Number.parseInt(rawValue ?? String(fallback), 10);
  return Number.isFinite(rawLimit) ? Math.min(Math.max(rawLimit, 1), 100) : fallback;
}

export function handleApiError(error: unknown): NextResponse {
  const host = process.env.WIKI_IC_HOST ?? "https://icp0.io";
  return NextResponse.json(classifyApiError(error, host), { status: 502 });
}
