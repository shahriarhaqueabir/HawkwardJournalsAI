import { invoke } from "../ipc.js";

export function initTasks() {
  const listEl = document.getElementById("task-list-items");
  const searchInput = document.getElementById("task-search");
  const newTaskBtn = document.getElementById("btn-new-task");
  
  // Detail Panel Elements
  const detailPanel = document.getElementById("task-detail-panel");
  const closeDetailBtn = document.getElementById("btn-close-task-detail");
  const saveTaskBtn = document.getElementById("btn-save-task");
  const deleteTaskBtn = document.getElementById("btn-delete-task");
  
  const detailId = document.getElementById("detail-task-id");
  const detailTitle = document.getElementById("detail-task-title");
  const detailDesc = document.getElementById("detail-task-desc");
  const detailPriority = document.getElementById("detail-task-priority");
  const detailStatus = document.getElementById("detail-task-status");
  const detailDate = document.getElementById("detail-task-date");
  const detailEnergy = document.getElementById("detail-task-energy");
  const detailProject = document.getElementById("detail-task-project");

  if (!listEl) return;

  // --- REFRESH / LIST ---
  async function refreshTasks(query = "") {
    try {
      const tasks = query.trim() 
        ? await invoke("task_search", { query })
        : await invoke("task_list");
      renderTasks(tasks);
    } catch (err) {
      console.error("Failed to load tasks:", err);
    }
  }

  function renderTasks(tasks) {
    if (!tasks || tasks.length === 0) {
      listEl.innerHTML = '<div class="list-empty">No tasks found.</div>';
      return;
    }

    listEl.innerHTML = tasks.map(task => {
      const priorityClass = `priority-${task.priority}`;
      const checked = task.status === 'done' ? 'checked' : '';
      
      return `
        <div class="task-item ${priorityClass}" data-id="${task.id}">
          <input type="checkbox" ${checked} class="task-toggle" title="Mark as done" />
          <div class="task-content">
            <div class="task-title">${task.title}</div>
            <div class="task-meta">
               ${task.project ? `<span class="task-project">${task.project}</span>` : ""}
               ${task.due_date ? `<span class="task-due">${task.due_date}</span>` : ""}
            </div>
          </div>
        </div>
      `;
    }).join("");

    // Toggle listener
    listEl.querySelectorAll(".task-toggle").forEach(cb => {
      cb.addEventListener("click", (e) => e.stopPropagation()); // Don't open panel when toggling
      cb.addEventListener("change", async (e) => {
        const id = e.target.closest(".task-item").dataset.id;
        const newStatus = e.target.checked ? "done" : "todo";
        await invoke("task_update_status", { id, status: newStatus });
        refreshTasks(searchInput.value);
      });
    });

    // Item click -> Open Detail
    listEl.querySelectorAll(".task-item").forEach(item => {
      item.addEventListener("click", () => openDetail(item.dataset.id));
    });
  }

  // --- DETAIL PANEL ---
  async function openDetail(id) {
    try {
      const task = await invoke("task_get", { id });
      if (!task) return;

      detailId.value = task.id;
      detailTitle.value = task.title;
      detailDesc.value = task.description || "";
      detailPriority.value = task.priority;
      detailStatus.value = task.status;
      detailDate.value = task.due_date || "";
      detailEnergy.value = task.energy_level || "";
      detailProject.value = task.project || "";

      detailPanel.classList.remove("hidden");
    } catch (err) {
      console.error("Failed to get task details:", err);
    }
  }

  function closeDetail() {
    detailPanel.classList.add("hidden");
  }

  // --- ACTIONS ---
  searchInput.addEventListener("input", (e) => {
    refreshTasks(e.target.value);
  });

  newTaskBtn.addEventListener("click", async () => {
    try {
      const id = await invoke("task_create", { title: "New Task" });
      refreshTasks(searchInput.value);
      openDetail(id);
    } catch (err) {
      console.error("Failed to create task:", err);
    }
  });

  saveTaskBtn.addEventListener("click", async () => {
    const task = {
      id: detailId.value,
      title: detailTitle.value,
      description: detailDesc.value || null,
      priority: detailPriority.value,
      status: detailStatus.value,
      due_date: detailDate.value || null,
      due_time: null, // Placeholder
      reminder_at: null,
      time_estimate: null,
      time_logged: 0,
      tags: "[]",
      labels: "[]",
      category: null,
      project: detailProject.value || null,
      energy_level: detailEnergy.value || null,
      context_tag: null,
      linked_url: null,
      ai_created: false,
      created_at: "", // Backend handles some, but we need to satisfy struct
      updated_at: "",
      completed_at: null
    };

    try {
      await invoke("task_update", { task });
      closeDetail();
      refreshTasks(searchInput.value);
    } catch (err) {
      console.error("Failed to update task:", err);
    }
  });

  deleteTaskBtn.addEventListener("click", async () => {
    if (!confirm("Are you sure you want to delete this task?")) return;
    try {
      await invoke("task_delete", { id: detailId.value });
      closeDetail();
      refreshTasks(searchInput.value);
    } catch (err) {
      console.error("Failed to delete task:", err);
    }
  });

  closeDetailBtn.addEventListener("click", closeDetail);

  // Initial load
  refreshTasks();
}
