// Where: extensions/conversation-capture/popup/popup.js
// What: Popup settings for the injected ChatGPT capture button.
// Why: Page capture now happens in the content UI; popup only stores destination config.
const canisterInput = document.querySelector("#canister-id");
const hostInput = document.querySelector("#host");
const saveButton = document.querySelector("#save-settings");
const statusText = document.querySelector("#status");

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
    canisterInput.value = response.config.canisterId;
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
    host: hostInput.value.trim() || "https://icp0.io"
  };
}
