import { initJournal } from "./tabs/journal.js";
import { initTasks } from "./tabs/tasks.js";
import { initAiChat } from "./tabs/ai.js";
import { initReports } from "./tabs/reports.js";
import { initSettings } from "./tabs/settings.js";
import { initAiSidebar } from "./ai-sidebar.js";
import { invoke } from "./ipc.js";

// Tab switching
document.querySelectorAll(".nav-item").forEach((item) => {
  item.addEventListener("click", () => {
    const tab = item.dataset.tab;
    const nextView = document.getElementById(`tab-${tab}`);
    if (!nextView) {
      console.warn(`Tab view not found: tab-${tab}`);
      return;
    }

    document
      .querySelectorAll(".nav-item")
      .forEach((n) => n.classList.remove("active"));
    item.classList.add("active");
    document
      .querySelectorAll(".tab-view")
      .forEach((v) => v.classList.remove("active"));
    nextView.classList.add("active");
  });
});

// Ollama health check
async function checkOllama() {
  const statusEl = document.getElementById("ollama-status");
  try {
    const online = await invoke("ollama_health_check", {}, { silent: true });
    if (online) {
      statusEl.textContent = "🟢 llama3.2";
      statusEl.style.color = "#4caf50";
    } else {
      statusEl.textContent = "🔴 Ollama offline";
      statusEl.style.color = "#f44336";
    }
  } catch (e) {
    console.warn("Ollama check failed:", e);
    statusEl.textContent = "🔴 Ollama offline";
  }
}

// Global Event Dispatcher (D-96)
globalThis.__TAURI__.event.listen("app_event", (event) => {
  const payload = event.payload;
  
  // 1. Pass to Journal Tab if mounted
  if (globalThis.__JOURNAL_EVENT_HANDLER__) {
    globalThis.__JOURNAL_EVENT_HANDLER__(payload);
  }

  // 2. Pass to AI Tab if mounted
  if (globalThis.__AI_CHAT_HANDLER__) {
    globalThis.__AI_CHAT_HANDLER__(payload);
  }

  // 3. Pass to AI Sidebar if mounted
  if (globalThis.__AI_SIDEBAR_HANDLER__) {
    globalThis.__AI_SIDEBAR_HANDLER__(payload);
  }

  // 4. Pass to Tasks Tab if mounted
  if (globalThis.__TASKS_EVENT_HANDLER__) {
    globalThis.__TASKS_EVENT_HANDLER__(payload);
  }

  // 5. Pass to Reports Tab if mounted
  if (globalThis.__REPORTS_EVENT_HANDLER__) {
    globalThis.__REPORTS_EVENT_HANDLER__(payload);
  }

  // 4. Global handlers (Toasts, System Status)
  if (payload.type === "database_error") {
    console.error(`[DB Error] ${payload.operation}: ${payload.error}`);
  }
});

// Initializations
initJournal();
initTasks();
initAiChat();
initReports();
initSettings();
initAiSidebar();
checkOllama();
setInterval(checkOllama, 30000);
