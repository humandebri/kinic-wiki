// Where: extensions/conversation-capture/popup/popup.js
// What: Popup settings for the injected ChatGPT capture button.
// Why: Page capture now happens in the content UI; popup only stores destination config.
const canisterInput = document.querySelector("#canister-id");
const databaseInput = document.querySelector("#database-id");
const hostInput = document.querySelector("#host");
const saveButton = document.querySelector("#save-settings");
const statusText = document.querySelector("#status");
const DEFAULT_CANISTER_ID = process.env.KINIC_CAPTURE_CANISTER_ID || "";
const DEFAULT_HOST = process.env.KINIC_CAPTURE_HOST || "http://127.0.0.1:8001";
const DEFAULT_DATABASE_ID = process.env.KINIC_CAPTURE_DATABASE_ID || "default";

saveButton.addEventListener("click", async () => {
  try {
    await send({ type: "save-config", config: currentConfig() });
    statusText.textContent = "Settings saved";
  } catch (error) {
    statusText.textContent = error instanceof Error ? error.message : String(error);
  }
});

load();

async function load() {
  try {
    const response = await send({ type: "load-config" });
    canisterInput.value = response.config.canisterId || DEFAULT_CANISTER_ID;
    databaseInput.value = response.config.databaseId || DEFAULT_DATABASE_ID;
    hostInput.value = response.config.host;
  } catch (error) {
    statusText.textContent = error instanceof Error ? error.message : String(error);
  }
}

async function send(message) {
  const response = await chrome.runtime.sendMessage(message);
  if (!response?.ok) {
    throw new Error(response?.error || "extension request failed");
  }
  return response;
}

function currentConfig() {
  return {
    canisterId: canisterInput.value.trim(),
    databaseId: databaseInput.value.trim() || DEFAULT_DATABASE_ID,
    host: hostInput.value.trim() || DEFAULT_HOST
  };
}
