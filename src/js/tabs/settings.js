import { invoke } from "../ipc.js";

const SETTING_KEYS = [
  "ai_model",
  "ai_ollama_url",
  "ai_context_window",
  "theme",
  "terminal_visible",
];

export async function initSettings() {
  const saveBtn = document.getElementById("btn-save-settings");
  if (!saveBtn) return;

  saveBtn.addEventListener("click", saveSettings);
  document.getElementById("setting-theme")?.addEventListener("change", (e) => {
    applyTheme(e.target.value);
  });

  await loadSettings();
}

async function loadSettings() {
  try {
    const settings = await invoke("settings_list");
    const map = new Map(settings.map((item) => [item.key, item.value]));

    document.getElementById("setting-ai-model").value = map.get("ai_model") || "llama3.2:latest";
    document.getElementById("setting-ai-url").value = map.get("ai_ollama_url") || "http://localhost:11434";
    document.getElementById("setting-ai-context-window").value = map.get("ai_context_window") || "16384";
    document.getElementById("setting-theme").value = map.get("theme") || "dark";
    document.getElementById("setting-terminal-visible").checked = (map.get("terminal_visible") || "false") === "true";

    applyTheme(document.getElementById("setting-theme").value);
  } catch (err) {
    console.error("Failed to load settings:", err);
  }
}

async function saveSettings() {
  const values = {
    ai_model: document.getElementById("setting-ai-model").value.trim() || "llama3.2:latest",
    ai_ollama_url: document.getElementById("setting-ai-url").value.trim() || "http://localhost:11434",
    ai_context_window: document.getElementById("setting-ai-context-window").value.trim() || "16384",
    theme: document.getElementById("setting-theme").value,
    terminal_visible: document.getElementById("setting-terminal-visible").checked ? "true" : "false",
  };

  try {
    for (const key of SETTING_KEYS) {
      await invoke("setting_set", { key, value: values[key] });
    }
    applyTheme(values.theme);
  } catch (err) {
    console.error("Failed to save settings:", err);
  }
}

function applyTheme(theme) {
  const root = document.documentElement;
  const stylesheet = document.getElementById("theme-stylesheet");
  root.setAttribute("data-theme", theme);
  if (stylesheet) {
    stylesheet.setAttribute("href", `styles/themes/${theme}.css`);
  }
}
