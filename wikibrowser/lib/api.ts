import { NextResponse } from "next/server";
import { validateCanisterId } from "@/lib/vfs-client";

export type RouteParams = Promise<{ canisterId: string }>;

export function invalidCanisterResponse(canisterId: string): NextResponse | null {
  const result = validateCanisterId(canisterId);
  if (typeof result !== "string") {
    return null;
  }
  return NextResponse.json({ error: `invalid canister id: ${result}` }, { status: 400 });
}

export function missingParam(name: string): NextResponse {
  return NextResponse.json({ error: `missing ${name}` }, { status: 400 });
}

export function clampLimit(rawValue: string | null, fallback: number): number {
  const rawLimit = Number.parseInt(rawValue ?? String(fallback), 10);
  return Number.isFinite(rawLimit) ? Math.min(Math.max(rawLimit, 1), 100) : fallback;
}

export function handleApiError(error: unknown): NextResponse {
  const message = error instanceof Error ? error.message : String(error);
  return NextResponse.json({ error: message }, { status: 502 });
}
