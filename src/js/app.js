import { initJournal } from "./tabs/journal.js";
import { initTasks } from "./tabs/tasks.js";
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
    statusEl.textContent = "🔴 Ollama offline";
  }
}

// Initializations
initJournal();
initTasks();
initAiSidebar();
checkOllama();
setInterval(checkOllama, 30000);
