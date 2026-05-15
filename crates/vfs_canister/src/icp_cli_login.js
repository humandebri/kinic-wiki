// Where: crates/vfs_canister/src/icp_cli_login.js
// What: Browser entrypoint for the canister-hosted ICP CLI login page.
// Why: The generated HTML must be self-contained while sharing II auth helpers.
import { AuthClient } from "@icp-sdk/auth/client";
import { DelegationChain, DelegationIdentity } from "@icp-sdk/core/identity";
import {
  CLI_DELEGATION_TTL_MS,
  CLI_DELEGATION_TTL_NS,
  authClientCreateOptions,
  decodeBase64UrlToBytes,
  identityProviderUrlForLocation,
  parseCliLoginHash
} from "../../../shared/ii-auth/index.js";

const status = document.querySelector("#status");
const error = document.querySelector("#error");
const button = document.querySelector("#login");
const request = parseCliLoginHash(location.hash);
let authClient = null;

if (!request) {
  fail("Invalid CLI login request.");
} else {
  history.replaceState(null, "", location.pathname);
  AuthClient.create(authClientCreateOptions(CLI_DELEGATION_TTL_MS))
    .then((client) => {
      authClient = client;
      button.disabled = false;
      status.textContent = "Continue to authorize this local CLI session.";
    })
    .catch((cause) => fail(errorMessage(cause)));
}

button.addEventListener("click", async () => {
  if (!authClient || !request) return;
  button.disabled = true;
  status.textContent = "Waiting for Internet Identity.";
  await authClient.login({
    identityProvider: identityProviderUrlForLocation(location),
    derivationOrigin: location.origin,
    maxTimeToLive: CLI_DELEGATION_TTL_NS,
    onSuccess: () => void postDelegation(authClient, request),
    onError: (cause) => fail(errorMessage(cause))
  });
});

async function postDelegation(client, nextRequest) {
  try {
    status.textContent = "Sending the delegation to the local CLI.";
    const identity = client.getIdentity();
    if (!(identity instanceof DelegationIdentity)) {
      throw new Error("Internet Identity delegation was not available.");
    }
    const sessionPublicKey = decodeBase64UrlToBytes(nextRequest.publicKey);
    const chain = await DelegationChain.create(
      identity,
      {
        toDer() {
          return sessionPublicKey;
        }
      },
      new Date(Date.now() + CLI_DELEGATION_TTL_MS),
      { previous: identity.getDelegation() }
    );
    const response = await fetch(nextRequest.callback, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(chain.toJSON()),
      redirect: "error"
    });
    if (!response.ok) {
      throw new Error(`CLI callback failed with HTTP ${response.status}.`);
    }
    await client.logout();
    status.textContent = "CLI login complete.";
    setTimeout(() => window.close(), 1000);
  } catch (cause) {
    fail(errorMessage(cause));
  }
}

function fail(message) {
  status.textContent = "CLI login failed.";
  error.hidden = false;
  error.textContent = message;
  button.disabled = true;
}

function errorMessage(cause) {
  return cause instanceof Error ? cause.message : String(cause);
}
