import { NextResponse, type NextRequest } from "next/server";

const APEX_HOST = "kinic.xyz";
const CANONICAL_HOST = "wiki.kinic.xyz";

export function middleware(request: NextRequest) {
  if (request.nextUrl.hostname !== APEX_HOST) {
    return NextResponse.next();
  }

  const url = request.nextUrl.clone();
  url.hostname = CANONICAL_HOST;

  return NextResponse.redirect(url, 308);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"]
};
