import { initJournal } from "./tabs/journal.js";
import { initTasks } from "./tabs/tasks.js";
import { initAiSidebar, handleAppEvent } from "./ai-sidebar.js";
import { invoke } from "./ipc.js";

// SINGLE typed event listener (D-96)
globalThis.__TAURI__.event.listen("app_event", (event) => {
  const payload = event.payload;
  // Dispatch to relevant modules
  handleAppEvent(payload);
});

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

// Partial Loading Helper
async function loadView(tabId, filePath) {
  const container = document.getElementById(`tab-${tabId}`);
  if (container.innerHTML.trim() === "" || container.innerHTML.includes("Loading...")) {
    try {
      const response = await fetch(filePath);
      const html = await response.text();
      container.innerHTML = html;
      return true;
    } catch (e) {
      console.error(`Failed to load view: ${filePath}`, e);
    }
  }
  return false;
}

// Initializations
initJournal();
initTasks();
initAiSidebar();
checkOllama();
setInterval(checkOllama, 30000);

// Load and Init Memory Map when tab is clicked
document.querySelector('[data-tab="memory"]').addEventListener('click', async () => {
  const loaded = await loadView('memory', 'views/memory.html');
  if (loaded || !window.memoryMapInstance) {
    // Import script dynamically
    const script = document.createElement('script');
    script.src = 'js/tabs/memory.js';
    script.onload = () => {
      window.memoryMapInstance = new window.MemoryMap();
    };
    document.head.appendChild(script);
  }
});

