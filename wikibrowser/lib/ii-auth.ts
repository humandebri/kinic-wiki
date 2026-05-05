"use client";

import { AuthClient } from "@dfinity/auth-client";
import type { Identity } from "@dfinity/agent";

type AuthState = {
  principal: string;
  authenticated: boolean;
};

const ANONYMOUS_PRINCIPAL = "2vxsx-fae";
let authClientPromise: Promise<AuthClient> | null = null;
const listeners = new Set<(state: AuthState) => void>();

export async function currentIdentity(): Promise<Identity | undefined> {
  const client = await authClient();
  if (!(await client.isAuthenticated())) {
    return undefined;
  }
  return client.getIdentity();
}

export async function currentAuthState(): Promise<AuthState> {
  const client = await authClient();
  const authenticated = await client.isAuthenticated();
  const principal = authenticated ? client.getIdentity().getPrincipal().toText() : ANONYMOUS_PRINCIPAL;
  return { principal, authenticated };
}

export function subscribeAuth(listener: (state: AuthState) => void): () => void {
  listeners.add(listener);
  currentAuthState().then(listener).catch(() => {
    listener({ principal: ANONYMOUS_PRINCIPAL, authenticated: false });
  });
  return () => {
    listeners.delete(listener);
  };
}

export async function loginWithInternetIdentity(): Promise<void> {
  const client = await authClient();
  await new Promise<void>((resolve, reject) => {
    client.login({
      identityProvider: identityProviderUrl(),
      maxTimeToLive: BigInt(8) * BigInt(3_600_000_000_000),
      onSuccess: () => resolve(),
      onError: (error) => reject(error)
    });
  });
  await notifyAuthListeners();
}

export async function logoutInternetIdentity(): Promise<void> {
  const client = await authClient();
  await client.logout();
  await notifyAuthListeners();
}

async function authClient(): Promise<AuthClient> {
  authClientPromise ??= AuthClient.create();
  return authClientPromise;
}

async function notifyAuthListeners(): Promise<void> {
  const state = await currentAuthState();
  for (const listener of listeners) {
    listener(state);
  }
  window.dispatchEvent(new Event("kinic-auth-change"));
}

function identityProviderUrl(): string {
  const host = window.location.hostname;
  if (host === "localhost" || host === "127.0.0.1" || host.endsWith(".localhost")) {
    return "http://id.ai.localhost:8000";
  }
  return "https://id.ai";
}
