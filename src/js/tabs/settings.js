import { invoke } from "../ipc.js";
import { showToast, showError } from "../notifications.js";

const SETTING_KEYS = [
  "ai_model",
  "ai_ollama_url",
  "ai_context_window",
  "ai_nudge_enabled",
  "theme",
  "terminal_visible",
];

let aiStatusInterval = null;

export async function initSettings() {
  const saveBtn = document.getElementById("btn-save-settings");
  if (!saveBtn) return;

  saveBtn.addEventListener("click", saveSettings);
  document.getElementById("setting-theme")?.addEventListener("change", (e) => {
    applyTheme(e.target.value);
  });

  // DB Controls
  document.getElementById("btn-db-backup")?.addEventListener("click", performBackup);
  document.getElementById("btn-db-export")?.addEventListener("click", exportJson);
  document.getElementById("btn-db-empty-trash")?.addEventListener("click", emptyTrash);
  document.getElementById("btn-db-reset")?.addEventListener("click", resetData);

  // System Visibility
  document.getElementById("btn-refresh-audit")?.addEventListener("click", refreshAuditLog);

  await loadSettings();
  await refreshAuditLog();
  
  // Start periodic status refresh
  if (aiStatusInterval) clearInterval(aiStatusInterval);
  aiStatusInterval = setInterval(refreshAIStatus, 5000);
  refreshAIStatus();
}

async function loadSettings() {
  try {
    const settings = await invoke("settings_list");
    const map = new Map(settings.map((item) => [item.key, item.value]));

    setVal("setting-ai-model", map.get("ai_model") || "llama3.2:latest");
    setVal("setting-ai-url", map.get("ai_ollama_url") || "http://localhost:11434");
    setVal("setting-ai-context-window", map.get("ai_context_window") || "32768");
    setCheck("setting-ai-nudge-enabled", map.get("ai_nudge_enabled") === "true");
    setVal("setting-theme", map.get("theme") || "dark");
    setCheck("setting-terminal-visible", map.get("terminal_visible") === "true");

    applyTheme(getVal("setting-theme"));

    // Load DB Path
    const dbPath = await invoke("db_get_path");
    setVal("setting-db-path", dbPath);
  } catch (err) {
    console.error("Failed to load settings:", err);
  }
}

async function saveSettings() {
  const values = {
    ai_model: getVal("setting-ai-model").trim() || "llama3.2:latest",
    ai_ollama_url: getVal("setting-ai-url").trim() || "http://localhost:11434",
    ai_context_window: getVal("setting-ai-context-window").trim() || "32768",
    ai_nudge_enabled: isChecked("setting-ai-nudge-enabled") ? "true" : "false",
    theme: getVal("setting-theme"),
    terminal_visible: isChecked("setting-terminal-visible") ? "true" : "false",
  };

  try {
    for (const key of SETTING_KEYS) {
      await invoke("setting_set", { key, value: values[key] });
    }
    applyTheme(values.theme);
    showToast("Settings saved successfully", { variant: "success" });
  } catch (err) {
    showError(err, { title: "Save Failed" });
  }
}

async function performBackup() {
  try {
    const path = await invoke("db_manual_backup");
    showToast("Backup created: " + path.split(/[\\\/]/).pop(), { variant: "success" });
  } catch (err) {
    showError(err, { title: "Backup Failed" });
  }
}

async function exportJson() {
  try {
    const path = await invoke("db_export_json");
    showToast("Data exported: " + path.split(/[\\\/]/).pop(), { variant: "success" });
  } catch (err) {
    showError(err, { title: "Export Failed" });
  }
}

async function emptyTrash() {
  if (!confirm("Are you sure you want to permanently delete all items in the trash?")) return;
  try {
    const count = await invoke("trash_empty");
    showToast(`Permanently deleted ${count} items`, { variant: "info" });
    await refreshAuditLog();
  } catch (err) {
    showError(err, { title: "Empty Trash Failed" });
  }
}

async function resetData() {
  const confirmed = confirm("CRITICAL: This will delete ALL journal entries, tasks, and projects. Are you absolutely sure?");
  if (!confirmed) return;
  
  const doublyConfirmed = prompt("Type 'RESET' to confirm deep wipe:");
  if (doublyConfirmed !== "RESET") return;

  try {
    await invoke("db_reset");
    showToast("Database reset successfully. Restarting session...", { variant: "success" });
    window.location.reload();
  } catch (err) {
    showError(err, { title: "Reset Failed" });
  }
}

async function refreshAuditLog() {
  const body = document.getElementById("audit-log-body");
  if (!body) return;

  try {
    const logs = await invoke("db_get_audit_log", { limit: 50 });
    body.innerHTML = logs.length ? logs.map(log => `
      <tr>
        <td>${log.action}</td>
        <td>${log.entity}</td>
        <td>${log.actor}</td>
        <td title="${log.created_at}">${timeSince(new Date(log.created_at))}</td>
      </tr>
    `).join("") : "<tr><td colspan='4'>No activity logged yet.</td></tr>";
  } catch (err) {
    body.innerHTML = "<tr><td colspan='4' class='error'>Failed to load logs</td></tr>";
  }
}

async function refreshAIStatus() {
  const badge = document.getElementById("ai-queue-badge");
  if (!badge) return;

  try {
    const status = await invoke("ai_get_queue_status");
    badge.textContent = `AI Queue: ${status.queue_length}`;
    badge.className = status.queue_length > 0 ? "badge badge--busy" : "badge";
  } catch (err) {
    console.error("AI Status poll failed:", err);
  }
}

// Helpers
function getVal(id) { return document.getElementById(id)?.value; }
function setVal(id, val) { const el = document.getElementById(id); if (el) el.value = val; }
function isChecked(id) { return document.getElementById(id)?.checked; }
function setCheck(id, val) { const el = document.getElementById(id); if (el) el.checked = !!val; }

function applyTheme(theme) {
  const root = document.documentElement;
  const stylesheet = document.getElementById("theme-stylesheet");
  root.dataset.theme = theme;
  if (stylesheet) {
    stylesheet.setAttribute("href", `styles/themes/${theme}.css`);
  }
}

function timeSince(date) {
  const seconds = Math.floor((new Date() - date) / 1000);
  let interval = seconds / 31536000;
  if (interval > 1) return Math.floor(interval) + "y ago";
  interval = seconds / 2592000;
  if (interval > 1) return Math.floor(interval) + "mo ago";
  interval = seconds / 86400;
  if (interval > 1) return Math.floor(interval) + "d ago";
  interval = seconds / 3600;
  if (interval > 1) return Math.floor(interval) + "h ago";
  interval = seconds / 60;
  if (interval > 1) return Math.floor(interval) + "m ago";
  return Math.floor(seconds) + "s ago";
}
