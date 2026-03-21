import { invoke } from "../ipc.js";


export function initTasks() {
  const columns = {
    todo: document.getElementById("tasks-todo"),
    in_progress: document.getElementById("tasks-in-progress"),
    done: document.getElementById("tasks-done"),
    cancelled: document.getElementById("tasks-cancelled")
  };
  const kanbanView = document.getElementById("tasks-view-kanban");
  const listView = document.getElementById("tasks-view-list");
  const calendarView = document.getElementById("tasks-view-calendar");
  const listViewEl = document.getElementById("tasks-list-view");
  const calendarViewEl = document.getElementById("tasks-calendar-view");
  const viewButtons = document.querySelectorAll("[data-task-view]");
  const searchInput = document.getElementById("task-search");
  const newTaskBtn = document.getElementById("btn-new-task");
  const projectFilter = document.getElementById("task-project-filter");
  const manageProjectsBtn = document.getElementById("btn-manage-projects");
  
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
  const subtaskList = document.getElementById("detail-task-subtasks");
  const addSubtaskBtn = document.getElementById("btn-add-subtask");
  const attachmentList  = document.getElementById("detail-task-attachments");
  const addAttachBtn    = document.getElementById("btn-add-attachment");
  const showProjectModalBtn = document.getElementById("btn-show-new-project");
  const projectCreateModal = document.getElementById("modal-project");
  const saveProjectBtn = document.getElementById("btn-create-project-save");
  const deleteProjectBtn = document.getElementById("btn-delete-project");
  const newProjectName = document.getElementById("new-project-name");
  const newProjectDesc = document.getElementById("new-project-desc");
  const editProjectId = document.getElementById("edit-project-id");
  const projectStatus = document.getElementById("project-status");
  const projectGoalDate = document.getElementById("project-goal-date");
  const projectColor = document.getElementById("project-color");
  const projectManagementList = document.getElementById("project-management-list");

  if (!columns.todo) return;

  let currentTask = null;
  let currentProjects = [];
  let draggedTaskId = null;
  let suppressClickUntil = 0;
  let activeView = "kanban";

  // Modals
  showProjectModalBtn?.addEventListener("click", () => {
    resetProjectForm();
    projectCreateModal.classList.remove("hidden");
  });
  manageProjectsBtn?.addEventListener("click", () => {
    resetProjectForm();
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
      const isEditing = Boolean(editProjectId.value);
      const project = {
        id: editProjectId.value || ("p_" + Math.random().toString(36).slice(2, 11)),
        name,
        description: newProjectDesc.value.trim() || null,
        status: projectStatus.value,
        color: projectColor.value || "#6c8ef7",
        goal_date: projectGoalDate.value || null,
        created_at: isEditing ? (currentProjects.find(p => p.id === editProjectId.value)?.created_at || now) : now,
        updated_at: now
      };
      if (isEditing) {
        await invoke("project_update", { project });
      } else {
        await invoke("project_create", { project });
      }
      resetProjectForm();
      await loadProjects();
    } catch (err) {
      console.error("Failed to create project:", err);
    }
  });

  deleteProjectBtn?.addEventListener("click", async () => {
    const id = editProjectId.value;
    if (!id || id === "inbox") return;
    if (!confirm("Delete this project? Tasks will be moved to Inbox.")) return;

    try {
      await invoke("project_delete", { id });
      resetProjectForm();
      await loadProjects();
      await refreshTasks(searchInput.value);
    } catch (err) {
      console.error("Failed to delete project:", err);
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
      renderListView(tasks);
      renderCalendarView(tasks);
    } catch (err) {
      console.error("Failed to load tasks:", err);
    }
  }

  async function loadProjects() {
    try {
      const projects = await invoke("project_list");
      currentProjects = projects;
      const currentFilter = projectFilter.value || "all";
      const currentDetailProject = detailProject.value || "inbox";
      const inboxOpt = '<option value="inbox">Inbox</option>';
      const filterHtml = '<option value="all">All Projects</option>' + inboxOpt +
        projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");
      const selectHtml = inboxOpt +
        projects.map(p => `<option value="${p.id}">${p.name}</option>`).join("");

      projectFilter.innerHTML = filterHtml;
      detailProject.innerHTML = selectHtml;
      projectFilter.value = [...projectFilter.options].some(opt => opt.value === currentFilter) ? currentFilter : "all";
      detailProject.value = [...detailProject.options].some(opt => opt.value === currentDetailProject) ? currentDetailProject : "inbox";
      renderProjectManagementList(projects);
    } catch (err) {
      console.error("Failed to load projects:", err);
    }
  }

  function resetProjectForm() {
    editProjectId.value = "";
    newProjectName.value = "";
    newProjectDesc.value = "";
    projectStatus.value = "active";
    projectGoalDate.value = "";
    projectColor.value = "#6c8ef7";
    saveProjectBtn.textContent = "Create Project";
    deleteProjectBtn.disabled = true;
  }

  function fillProjectForm(project) {
    editProjectId.value = project.id;
    newProjectName.value = project.name || "";
    newProjectDesc.value = project.description || "";
    projectStatus.value = project.status || "active";
    projectGoalDate.value = project.goal_date || "";
    projectColor.value = project.color || "#6c8ef7";
    saveProjectBtn.textContent = "Save Project";
    deleteProjectBtn.disabled = project.id === "inbox";
    projectCreateModal.classList.remove("hidden");
  }

  function renderProjectManagementList(projects) {
    if (!projectManagementList) return;
    if (!projects || projects.length === 0) {
      projectManagementList.innerHTML = '<div class="list-empty">No projects</div>';
      return;
    }

    projectManagementList.innerHTML = projects.map(project => `
      <div class="project-management-item" data-project-edit="${project.id}">
        <div class="project-management-main">
          <div class="project-management-title">${escapeHtml(project.name)}</div>
          <div class="project-management-meta">${escapeHtml(project.status)}${project.goal_date ? ` · ${escapeHtml(project.goal_date)}` : ""}</div>
        </div>
        <div class="project-management-actions">
          <button class="btn-linkish" data-project-edit="${project.id}">Edit</button>
        </div>
      </div>
    `).join("");

    projectManagementList.querySelectorAll("[data-project-edit]").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const id = btn.dataset.projectEdit;
        try {
          const project = await invoke("project_get", { id });
          if (project) fillProjectForm(project);
        } catch (err) {
          console.error("Failed to load project:", err);
        }
      });
    });
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
      item.addEventListener("click", () => {
        if (Date.now() < suppressClickUntil) return;
        openDetail(item.dataset.id);
      });
      
      // Basic Drag and Drop (native)
      item.addEventListener("dragstart", (e) => {
        draggedTaskId = item.dataset.id;
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/plain", item.dataset.id);
        item.classList.add("dragging");
      });
      item.addEventListener("dragend", () => {
        draggedTaskId = null;
        suppressClickUntil = Date.now() + 200;
        item.classList.remove("dragging");
      });
    });
  }

  function renderListView(tasks) {
    if (!listViewEl) return;
    if (!tasks || tasks.length === 0) {
      listViewEl.innerHTML = '<div class="list-empty">No tasks</div>';
      return;
    }

    const groups = [
      ["todo", "To Do"],
      ["in_progress", "In Progress"],
      ["done", "Done"],
      ["cancelled", "Cancelled"],
    ];

    listViewEl.innerHTML = groups.map(([status, label]) => {
      const items = tasks.filter(task => task.status === status);
      const rows = items.length
        ? items.map(renderTaskCard).join("")
        : '<div class="list-empty">No tasks</div>';
      return `
        <section class="tasks-list-group">
          <h3>${label}</h3>
          <div class="tasks-list-items">${rows}</div>
        </section>
      `;
    }).join("");

    bindTaskCards(listViewEl);
  }

  function renderCalendarView(tasks) {
    if (!calendarViewEl) return;

    const datedTasks = (tasks || [])
      .filter(task => task.due_date)
      .sort((a, b) => String(a.due_date).localeCompare(String(b.due_date)));

    if (datedTasks.length === 0) {
      calendarViewEl.innerHTML = '<div class="list-empty">No tasks with due dates</div>';
      return;
    }

    const groups = new Map();
    datedTasks.forEach((task) => {
      const key = task.due_date;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(task);
    });

    calendarViewEl.innerHTML = Array.from(groups.entries()).map(([date, items]) => `
      <section class="tasks-calendar-day">
        <h3>${formatCalendarDate(date)}</h3>
        <div class="tasks-calendar-items">
          ${items.map(renderTaskCard).join("")}
        </div>
      </section>
    `).join("");

    bindTaskCards(calendarViewEl);
  }

  function renderTaskCard(task) {
    const priorityClass = `priority-${task.priority}`;
    return `
      <div class="task-item ${priorityClass}" data-id="${task.id}" draggable="false">
        <input type="checkbox" ${task.status === "done" ? "checked" : ""} class="task-toggle" title="Mark as done" />
        <div class="task-content">
          <div class="task-title">${escapeHtml(task.title)}</div>
          <div class="task-meta">
            ${task.project ? `<span class="task-project">${escapeHtml(task.project)}</span>` : ""}
            ${task.due_date ? `<span class="task-due">${escapeHtml(task.due_date)}</span>` : ""}
          </div>
        </div>
      </div>
    `;
  }

  function bindTaskCards(root) {
    root.querySelectorAll(".task-toggle").forEach(cb => {
      cb.addEventListener("click", (e) => e.stopPropagation());
      cb.addEventListener("change", async (e) => {
        const id = e.target.closest(".task-item").dataset.id;
        const newStatus = e.target.checked ? "done" : "todo";
        await invoke("task_update_status", { id, status: newStatus });
        refreshTasks(searchInput.value);
      });
    });

    root.querySelectorAll(".task-item").forEach((item) => {
      item.addEventListener("click", () => openDetail(item.dataset.id));
    });
  }

  function setActiveView(nextView) {
    activeView = nextView;
    const views = {
      kanban: kanbanView,
      list: listView,
      calendar: calendarView,
    };
    Object.entries(views).forEach(([name, panel]) => {
      panel?.classList.toggle("active", name === nextView);
    });
    viewButtons.forEach((button) => {
      button.classList.toggle("active", button.dataset.taskView === nextView);
    });
  }

  async function loadSubtasks(taskId) {
    if (!subtaskList) return;

    try {
      const tasks = await invoke("task_list", { filters: { exclude_statuses: [] } });
      const subtasks = tasks.filter(task => task.parent_task_id === taskId);
      renderSubtasks(subtasks);
    } catch (err) {
      console.error("Failed to load subtasks:", err);
      subtaskList.innerHTML = '<div class="subtask-empty">Failed to load subtasks</div>';
    }
  }

  function renderSubtasks(subtasks) {
    if (!subtaskList) return;
    if (!subtasks || subtasks.length === 0) {
      subtaskList.innerHTML = '<div class="subtask-empty">No subtasks</div>';
      return;
    }

    subtaskList.innerHTML = subtasks.map((task) => `
      <div class="subtask-item" data-subtask-id="${task.id}">
        <input type="checkbox" class="task-toggle" ${task.status === "done" ? "checked" : ""} />
        <div class="subtask-main">
          <div class="subtask-title">${escapeHtml(task.title)}</div>
          <div class="subtask-meta">${escapeHtml(task.status)}${task.due_date ? ` · ${escapeHtml(task.due_date)}` : ""}</div>
        </div>
        <div class="subtask-actions">
          <button class="btn-linkish" data-subtask-open="${task.id}">Open</button>
          <button class="btn-linkish" data-subtask-delete="${task.id}">Delete</button>
        </div>
      </div>
    `).join("");

    subtaskList.querySelectorAll("[data-subtask-open]").forEach((btn) => {
      btn.addEventListener("click", () => openDetail(btn.dataset.subtaskOpen));
    });

    subtaskList.querySelectorAll("[data-subtask-delete]").forEach((btn) => {
      btn.addEventListener("click", async () => {
        if (!confirm("Delete this subtask?")) return;
        try {
          await invoke("task_delete", { id: btn.dataset.subtaskDelete });
          if (currentTask) {
            await loadSubtasks(currentTask.id);
            await refreshTasks(searchInput.value);
          }
        } catch (err) {
          console.error("Failed to delete subtask:", err);
        }
      });
    });

    subtaskList.querySelectorAll(".task-toggle").forEach((checkbox) => {
      checkbox.addEventListener("change", async (event) => {
        const id = event.target.closest(".subtask-item").dataset.subtaskId;
        const status = event.target.checked ? "done" : "todo";
        try {
          await invoke("task_update_status", { id, status });
          if (currentTask) {
            await loadSubtasks(currentTask.id);
            await refreshTasks(searchInput.value);
          }
        } catch (err) {
          console.error("Failed to update subtask status:", err);
        }
      });
    });
  }

  // Setup Column Drop Target once
  Object.entries(columns).forEach(([status, col]) => {
    let dragDepth = 0;

    col.addEventListener("dragenter", (e) => {
      e.preventDefault();
      dragDepth += 1;
      col.classList.add("drag-over");
    });
    col.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      col.classList.add("drag-over");
    });
    col.addEventListener("dragleave", () => {
      dragDepth = Math.max(0, dragDepth - 1);
      if (dragDepth === 0) {
        col.classList.remove("drag-over");
      }
    });
    col.addEventListener("drop", async (e) => {
      e.preventDefault();
      dragDepth = 0;
      col.classList.remove("drag-over");
      const id = e.dataTransfer.getData("text/plain") || draggedTaskId;
      if (id) {
        const existingTask = document.querySelector(`.task-item[data-id="${id}"]`);
        const currentStatus = existingTask?.closest(".column-items")?.dataset.status || null;
        if (currentStatus === status) {
          return;
        }

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
      addSubtaskBtn.disabled = Boolean(task.parent_task_id);
      addSubtaskBtn.title = task.parent_task_id
        ? "Subtasks can only be created one level deep"
        : "Add a subtask";

      detailPanel.classList.remove("hidden");
      await loadAttachments(task.id);
      await loadSubtasks(task.id);
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

  viewButtons.forEach((button) => {
    button.addEventListener("click", () => {
      setActiveView(button.dataset.taskView || "kanban");
    });
  });

  projectFilter.addEventListener("change", () => {
    refreshTasks(searchInput.value);
  });

  newTaskBtn.addEventListener("click", async () => {
    try {
      const payload = { title: "New Task" };
      if (projectFilter.value && projectFilter.value !== "all") {
        payload.projectId = projectFilter.value;
      }
      const task = await invoke("task_create", payload);
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

  addSubtaskBtn?.addEventListener("click", async () => {
    if (!currentTask || currentTask.parent_task_id) return;

    try {
      const task = await invoke("task_create", {
        title: "New Subtask",
        parentTaskId: currentTask.id,
        projectId: currentTask.project_id || "inbox"
      });
      await refreshTasks(searchInput.value);
      await loadSubtasks(currentTask.id);
      openDetail(task.id);
    } catch (err) {
      console.error("Failed to create subtask:", err);
    }
  });

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
  setActiveView("kanban");

  globalThis.__TASKS_EVENT_HANDLER__ = (payload) => {
    if (["task_created", "task_updated", "task_deleted", "task_completed", "task_restored"].includes(payload.type)) {
      refreshTasks(searchInput.value);
      if (currentTask && (payload.id === currentTask.id || payload.task_id === currentTask.id)) {
        openDetail(currentTask.id);
      }
    }
  };

  function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text ?? "";
    return div.innerHTML;
  }

  function formatCalendarDate(dateValue) {
    const date = new Date(`${dateValue}T12:00:00`);
    if (Number.isNaN(date.getTime())) {
      return dateValue;
    }
    return date.toLocaleDateString(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }
}
