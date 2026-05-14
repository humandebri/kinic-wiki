// Where: wikibrowser/app/.well-known/ii-alternative-origins/route.ts
// What: Internet Identity alternative origins for the Chrome extension.
// Why: The extension derives the same principal as wiki.kinic.xyz.
const ALTERNATIVE_ORIGINS = [
  "chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj",
  "chrome-extension://hbnicbmdodpmihmcnfgejcdgbfmemoci",
];

export function GET() {
  return Response.json(
    { alternativeOrigins: ALTERNATIVE_ORIGINS },
    {
      headers: {
        "access-control-allow-origin": "*",
        "cache-control": "public, max-age=300"
      }
    }
  );
}
