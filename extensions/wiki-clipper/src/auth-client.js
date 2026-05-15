// Where: extensions/wiki-clipper/src/auth-client.js
// What: Internet Identity AuthClient helpers for extension UI/offscreen pages.
// Why: MV3 service workers lack a stable window context for AuthClient.
import { AuthClient } from "@icp-sdk/auth/client";

const NS_PER_DAY = BigInt(24 * 60 * 60) * BigInt(1_000_000_000);
const AUTH_DAYS = 29;
export const DERIVATION_ORIGIN = "https://xis3j-paaaa-aaaai-axumq-cai.icp0.io";

export const AUTH_OPTIONS = {
  createOptions: {
    idleOptions: {
      idleTimeout: AUTH_DAYS * 24 * 60 * 60 * 1000,
      disableDefaultIdleCallback: true
    }
  },
  loginOptions: {
    identityProvider: "https://id.ai",
    derivationOrigin: DERIVATION_ORIGIN,
    maxTimeToLive: BigInt(AUTH_DAYS) * NS_PER_DAY,
    windowOpenerFeatures: "toolbar=0,location=0,menubar=0,width=500,height=500,left=100,top=100"
  }
};

let clientPromise = null;

export function getAuthClient() {
  clientPromise ??= AuthClient.create(AUTH_OPTIONS.createOptions);
  return clientPromise;
}

export async function authSnapshot() {
  const client = await getAuthClient();
  const isAuthenticated = await client.isAuthenticated();
  const identity = isAuthenticated ? client.getIdentity() : null;
  return {
    isAuthenticated,
    identity,
    principal: identity ? identity.getPrincipal().toText() : null
  };
}

export async function loginWithInternetIdentity() {
  const client = await getAuthClient();
  await new Promise((resolve, reject) => {
    client.login({
      ...AUTH_OPTIONS.loginOptions,
      onSuccess: resolve,
      onError: (error) => reject(new Error(String(error)))
    });
  });
  return authSnapshot();
}

export async function logoutInternetIdentity() {
  const client = await getAuthClient();
  await client.logout();
  return authSnapshot();
}
