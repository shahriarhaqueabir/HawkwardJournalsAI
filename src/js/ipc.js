import { showError } from "./notifications.js";

function formatIpcError(err, command) {
  const msg =
    typeof err === "string"
      ? err
      : typeof err?.message === "string"
        ? err.message
        : String(err);
  return `${command}: ${msg}`;
}

export function invoke(command, args = {}, { silent = false } = {}) {
  const tauriInvoke = window?.__TAURI__?.core?.invoke;
  
  if (typeof tauriInvoke !== "function") {
    const isBrowser = !window.hasOwnProperty("__TAURI_INTERNALS__") && !window.hasOwnProperty("__TAURI__");
    const msg = isBrowser 
      ? "Tauri invoke() is unavailable. Are you viewing this in a browser instead of the Tauri app window?"
      : "Tauri invoke() is unavailable. Check tauri.conf.json > withGlobalTauri: true.";
    
    const e = new Error(msg);
    if (!silent) showError(e, { title: "IPC Connectivity Error" });
    return Promise.reject(e);
  }

  return tauriInvoke(command, args).catch((err) => {
    if (!silent) showError(formatIpcError(err, command), { title: "IPC error" });
    throw err;
  });
}
