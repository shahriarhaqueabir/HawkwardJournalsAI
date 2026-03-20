import { invoke } from "../ipc.js";

export function initTasks() {
  const columns = {
    todo: document.getElementById("tasks-todo"),
    in_progress: document.getElementById("tasks-in-progress"),
    done: document.getElementById("tasks-done")
  };
  const searchInput = document.getElementById("task-search");
  const newTaskBtn = document.getElementById("btn-new-task");
  const projectFilter = document.getElementById("task-project-filter");
  
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
  const detailRecurrence = document.getElementById("detail-task-recurrence");

  if (!columns.todo) return;

  // --- REFRESH / LIST ---
  async function refreshTasks(query = "") {
    try {
      let tasks = query.trim() 
        ? await invoke("task_search", { query })
        : await invoke("task_list");

      // Client-side project filter (could be backend later)
      const filter = projectFilter.value;
      if (filter !== "all") {
        tasks = tasks.filter(t => t.project_id === filter);
      }

      renderTasks(tasks);
    } catch (err) {
      console.error("Failed to load tasks:", err);
    }
  }

  async function loadProjects() {
    try {
      const projects = await invoke("project_list");
      const filterHtml = '<option value="all">All Projects</option>' + 
        projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");
      const selectHtml = projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");
      
      projectFilter.innerHTML = filterHtml;
      detailProject.innerHTML = selectHtml;
    } catch (err) {
      console.error("Failed to load projects:", err);
    }
  }

  function renderTasks(tasks) {
    // Clear columns
    Object.values(columns).forEach(col => {
      col.innerHTML = "";
    });

    if (!tasks || tasks.length === 0) {
      Object.values(columns).forEach(col => {
        col.innerHTML = '<div class="list-empty">No tasks</div>';
      });
      return;
    }

    tasks.forEach(task => {
      const priorityClass = `priority-${task.priority}`;
      const checked = task.status === 'done' ? 'checked' : '';
      
      const html = `
        <div class="task-item ${priorityClass}" data-id="${task.id}" draggable="true">
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

      const targetCol = columns[task.status] || columns.todo;
      targetCol.insertAdjacentHTML("beforeend", html);
    });

    // Add empty state if column became empty
    Object.values(columns).forEach(col => {
      if (col.innerHTML === "") {
        col.innerHTML = '<div class="list-empty">No tasks</div>';
      }
    });

    // Toggle listener
    document.querySelectorAll(".task-toggle").forEach(cb => {
      cb.addEventListener("click", (e) => e.stopPropagation());
      cb.addEventListener("change", async (e) => {
        const id = e.target.closest(".task-item").dataset.id;
        const newStatus = e.target.checked ? "done" : "todo";
        await invoke("task_update_status", { id, status: newStatus });
        refreshTasks(searchInput.value);
      });
    });

    // Item click -> Open Detail
    document.querySelectorAll(".task-item").forEach(item => {
      item.addEventListener("click", () => openDetail(item.dataset.id));
      
      // Basic Drag and Drop (native)
      item.addEventListener("dragstart", (e) => {
        e.dataTransfer.setData("text/plain", item.dataset.id);
        item.classList.add("dragging");
      });
      item.addEventListener("dragend", () => {
        item.classList.remove("dragging");
      });
    });

    // Column drop listeners (setup once or keep if re-rendered?)
    // Actually better to setup once in initTasks if possible, but cards are re-rendered.
  }

  // Setup Column Drop Target once
  Object.entries(columns).forEach(([status, col]) => {
    col.addEventListener("dragover", (e) => {
      e.preventDefault();
      col.classList.add("drag-over");
    });
    col.addEventListener("dragleave", () => {
      col.classList.remove("drag-over");
    });
    col.addEventListener("drop", async (e) => {
      e.preventDefault();
      col.classList.remove("drag-over");
      const id = e.dataTransfer.getData("text/plain");
      if (id) {
        await invoke("task_update_status", { id, status });
        refreshTasks(searchInput.value);
      }
    });
  });

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
      detailProject.value = task.project_id || "inbox";

      detailRecurrence.value = task.recurrence || "";

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

  projectFilter.addEventListener("change", () => {
    refreshTasks(searchInput.value);
  });

  newTaskBtn.addEventListener("click", async () => {
    try {
      const task = await invoke("task_create", { title: "New Task" });
      refreshTasks(searchInput.value);
      openDetail(task.id);
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
      due_time: null,
      reminder_at: null,
      time_estimate: null,
      time_logged: 0,
      tags: "[]",
      labels: "[]",
      category: null,
      project: detailProject.value || null,
      recurrence: detailRecurrence.value || null,
      next_occurrence: null,
      energy_level: detailEnergy.value || null,
      context_tag: null,
      linked_url: null,
      ai_created: false,
      created_at: "", 
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
  loadProjects().then(() => refreshTasks());
}
