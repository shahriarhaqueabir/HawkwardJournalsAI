import { invoke } from "../ipc.js";


export function initTasks() {
  const columns = {
    todo: document.getElementById("tasks-todo"),
    in_progress: document.getElementById("tasks-in-progress"),
    done: document.getElementById("tasks-done"),
    cancelled: document.getElementById("tasks-cancelled")
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
  const detailContext = document.getElementById("detail-task-context");
  const detailEstimate = document.getElementById("detail-task-estimate");
  const detailUrl = document.getElementById("detail-task-url");
  const detailNotes = document.getElementById("detail-task-notes");
  const attachmentList  = document.getElementById("detail-task-attachments");
  const addAttachBtn    = document.getElementById("btn-add-attachment");
  const showProjectModalBtn = document.getElementById("btn-show-new-project");
  const projectCreateModal = document.getElementById("modal-project");
  const saveProjectBtn = document.getElementById("btn-create-project-save");
  const newProjectName = document.getElementById("new-project-name");
  const newProjectDesc = document.getElementById("new-project-desc");

  if (!columns.todo) return;

  let currentTask = null;

  // Modals
  showProjectModalBtn?.addEventListener("click", () => {
    newProjectName.value = "";
    newProjectDesc.value = "";
    projectCreateModal.classList.remove("hidden");
  });

  document.querySelectorAll("[data-modal-close]").forEach(btn => {
    btn.addEventListener("click", () => {
      btn.closest(".modal-overlay").classList.add("hidden");
    });
  });

  saveProjectBtn?.addEventListener("click", async () => {
    const name = newProjectName.value.trim();
    if (!name) return;

    try {
      const now = new Date().toISOString();
      const project = {
        id: "p_" + Math.random().toString(36).slice(2, 11),
        name,
        description: newProjectDesc.value.trim() || null,
        status: "active",
        color: "#6c8ef7",
        created_at: now,
        updated_at: now
      };
      await invoke("project_create", { project });
      projectCreateModal.classList.add("hidden");
      await loadProjects();
    } catch (err) {
      console.error("Failed to create project:", err);
    }
  });

  // --- REFRESH / LIST ---
  async function refreshTasks(query = "") {
    try {
      const filter = projectFilter.value;
      let tasks;
      
      if (query.trim()) {
        tasks = await invoke("task_search", { query });
        if (filter !== "all") {
          tasks = tasks.filter(t => t.project_id === filter);
        }
      } else {
        const payload = { filters: { exclude_statuses: [] } };
        if (filter !== "all") {
          payload.filters.project_id = filter;
        }
        tasks = await invoke("task_list", payload);
      }

      renderTasks(tasks);
    } catch (err) {
      console.error("Failed to load tasks:", err);
    }
  }

  async function loadProjects() {
    try {
      const projects = await invoke("project_list");
      const inboxOpt = '<option value="inbox">Inbox</option>';
      const filterHtml = '<option value="all">All Projects</option>' + inboxOpt +
        projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");
      const selectHtml = inboxOpt +
        projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");

      projectFilter.innerHTML = filterHtml;
      detailProject.innerHTML = selectHtml;
    } catch (err) {
      console.error("Failed to load projects:", err);
    }
  }

  // --- ATTACHMENTS ---
  async function loadAttachments(taskId) {
    try {
      const attachments = await invoke("attachment_list", { taskId: taskId });
      renderAttachments(attachments, taskId);
    } catch (err) {
      console.error("Failed to load attachments:", err);
      attachmentList.innerHTML = '<div class="attachment-empty">Failed to load</div>';
    }
  }

  function renderAttachments(attachments, taskId) {
    if (!attachments || attachments.length === 0) {
      attachmentList.innerHTML = '<div class="attachment-empty">No attachments</div>';
      return;
    }

    attachmentList.innerHTML = attachments.map(att => {
      let sizeLabel = "";
      if (att.size_bytes) {
        sizeLabel = att.size_bytes < 1024 * 1024
          ? `${(att.size_bytes / 1024).toFixed(1)} KB`
          : `${(att.size_bytes / (1024 * 1024)).toFixed(1)} MB`;
      }
      const missingClass = att.file_missing ? " missing" : "";
      const missingBadge = att.file_missing ? " ⚠️" : "";
      return `
        <div class="attachment-chip${missingClass}" data-att-id="${att.id}">
          <span class="attachment-chip-name" title="${att.file_name}">${att.file_name}${missingBadge}</span>
          ${sizeLabel ? `<span class="attachment-chip-size">${sizeLabel}</span>` : ""}
          <button class="attachment-chip-del" data-att-id="${att.id}" data-task-id="${taskId}" title="Remove attachment" aria-label="Remove attachment">×</button>
        </div>`;
    }).join("");

    // Wire delete buttons
    attachmentList.querySelectorAll(".attachment-chip-del").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        const attId  = btn.dataset.attId;
        const tskId  = btn.dataset.taskId;
        try {
          await invoke("attachment_remove", { id: attId, taskId: tskId });
          await loadAttachments(tskId);
        } catch (err) {
          console.error("Failed to remove attachment:", err);
        }
      });
    });
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
      currentTask = task;

      detailId.value = task.id;
      detailTitle.value = task.title;
      detailDesc.value = task.description || "";
      detailPriority.value = task.priority;
      detailStatus.value = task.status;
      detailDate.value = task.due_date || "";
      detailEnergy.value = task.energy_level || "";
      detailProject.value = task.project_id || "inbox";
      detailRecurrence.value = task.recurrence || "";
      detailContext.value = task.context_tag || "";
      detailEstimate.value = task.time_estimate || "";
      detailUrl.value = task.linked_url || "";
      detailNotes.value = task.notes || "";

      detailPanel.classList.remove("hidden");
      await loadAttachments(task.id);
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
    if (!currentTask) return;

    const updatedTask = {
      ...currentTask,
      title: detailTitle.value,
      description: detailDesc.value || null,
      priority: detailPriority.value,
      status: detailStatus.value,
      due_date: detailDate.value || null,
      project_id: detailProject.value || "inbox",
      recurrence: detailRecurrence.value || null,
      energy_level: detailEnergy.value || null,
      context_tag: detailContext.value || null,
      time_estimate: detailEstimate.value ? Number.parseInt(detailEstimate.value, 10) : null,
      linked_url: detailUrl.value || null,
      notes: detailNotes.value || null,
    };

    try {
      await invoke("task_update", { task: updatedTask });
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

  // --- ADD ATTACHMENT ---
  addAttachBtn.addEventListener("click", async () => {
    if (!currentTask) return;
    try {
      // Tauri v2 dialog plugin is injected on globalThis.__TAURI__.dialog
      const dialog = globalThis.__TAURI__?.dialog;
      if (!dialog) { console.error("Tauri dialog plugin not available"); return; }
      const selected = await dialog.open({
        multiple: false,
        title: "Select Attachment"
      });
      if (!selected) return; // user cancelled
      const sourcePath = typeof selected === "string" ? selected : selected.path;
      await invoke("attachment_add", { taskId: currentTask.id, sourcePath: sourcePath });
      await loadAttachments(currentTask.id);
    } catch (err) {
      console.error("Failed to add attachment:", err);
    }
  });

  // Initial load
  loadProjects().then(() => refreshTasks());

  globalThis.__TASKS_EVENT_HANDLER__ = (payload) => {
    if (["task_created", "task_updated", "task_deleted", "task_completed", "task_restored"].includes(payload.type)) {
      refreshTasks(searchInput.value);
      if (currentTask && (payload.id === currentTask.id || payload.task_id === currentTask.id)) {
        openDetail(currentTask.id);
      }
    }
  };
}
