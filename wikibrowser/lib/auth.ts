export const DELEGATION_TTL_NS = BigInt(8) * BigInt(3_600_000_000_000);

export function identityProviderUrl(): string {
  if (process.env.NEXT_PUBLIC_II_PROVIDER_URL) {
    return process.env.NEXT_PUBLIC_II_PROVIDER_URL;
  }
  const host = window.location.hostname;
  if (host === "localhost" || host === "127.0.0.1" || host.endsWith(".localhost")) {
    return "http://id.ai.localhost:8000";
  }
  return "https://id.ai";
}
