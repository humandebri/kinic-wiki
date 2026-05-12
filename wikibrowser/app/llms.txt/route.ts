import { listAeoPageLinks } from "@/lib/aeo/pages";
import { absoluteUrl } from "@/lib/aeo/site";

export const runtime = "nodejs";
export const revalidate = 86400;

export function GET() {
  const links = listAeoPageLinks()
    .map((page) => `- ${page.title}: ${absoluteUrl(page.path)}`)
    .join("\n");
  return new Response(`# Kinic\n\nKinic is an AI memory for browsing and accessing important information in one organized place.\n\n## Key pages\n\n${links}\n`, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8"
    }
  });
}
