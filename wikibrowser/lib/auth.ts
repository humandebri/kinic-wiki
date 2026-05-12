const HOURS_PER_DAY = BigInt(24);
const NANOSECONDS_PER_HOUR = BigInt(3_600_000_000_000);

export const DELEGATION_TTL_NS = BigInt(30) * HOURS_PER_DAY * NANOSECONDS_PER_HOUR;

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
