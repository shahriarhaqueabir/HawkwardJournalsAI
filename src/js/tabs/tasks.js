import { invoke } from "../ipc.js";

export function initTasks() {
  const listEl = document.getElementById("task-list-items");
  const searchInput = document.getElementById("task-search");
  const newTaskBtn = document.getElementById("btn-new-task");
  const taskTabs = document.querySelectorAll(".task-tab");
  const viewTabs = document.querySelectorAll(".view-tab");
  const listContainer = document.getElementById("task-list-items");
  const boardContainer = document.getElementById("task-kanban-board");
  
  let currentView = "list";
  
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

  if (!listEl) return;
  
  let currentFilter = "all";

  // --- REFRESH / LIST ---
  async function refreshTasks(query = "") {
    try {
      let tasks = query.trim() 
        ? await invoke("task_search", { query })
        : await invoke("task_list");
      
      // Client-side filtering for sub-tabs
      if (currentFilter === "pending") {
        tasks = tasks.filter(t => t.status !== 'done' && t.status !== 'cancelled');
      } else if (currentFilter === "completed") {
        tasks = tasks.filter(t => t.status === 'done');
      }

      if (currentView === "list") {
        renderTasks(tasks);
        listContainer.classList.remove("hidden");
        boardContainer.classList.add("hidden");
      } else {
        renderKanban(tasks);
        listContainer.classList.add("hidden");
        boardContainer.classList.remove("hidden");
      }
    } catch (err) {
      console.error("Failed to load tasks:", err);
    }
  }

  function renderKanban(tasks) {
    const columns = boardContainer.querySelectorAll(".column-items");
    columns.forEach(c => c.innerHTML = "");

    tasks.forEach(task => {
      let status = task.status;
      if (status === 'cancelled') return; // Don't show in kanban
      
      const col = boardContainer.querySelector(`.kanban-column[data-status="${status}"] .column-items`);
      if (col) {
        const card = document.createElement("div");
        card.className = `kanban-card priority-${task.priority}`;
        card.draggable = true;
        card.dataset.id = task.id;
        card.innerHTML = `
          <div class="card-title">${task.title}</div>
          <div class="card-meta">
            ${task.project ? `<span>#${task.project}</span>` : ""}
            ${task.is_blocked ? "<span>🔒</span>" : ""}
          </div>
        `;
        
        card.addEventListener("click", () => openDetail(task.id));
        card.addEventListener("dragstart", (e) => {
          card.classList.add("dragging");
          e.dataTransfer.setData("text/plain", task.id);
        });
        card.addEventListener("dragend", () => {
          card.classList.remove("dragging");
        });
        
        col.appendChild(card);
      }
    });

    // Setup drop zones
    boardContainer.querySelectorAll(".kanban-column").forEach(col => {
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
        const newStatus = col.dataset.status;
        
        await invoke("task_update_status", { id, status: newStatus });
        refreshTasks(searchInput.value);
      });
    });
  }

  function renderTasks(tasks) {
    if (!tasks || tasks.length === 0) {
      listEl.innerHTML = '<div class="list-empty">No tasks found.</div>';
      return;
    }

    // Sort/Organize tasks into a flat list with children following parents
    const taskMap = new Map(tasks.map(t => [t.id, { ...t, children: [] }]));
    const roots = [];
    
    taskMap.forEach(task => {
      if (task.parent_task_id && taskMap.has(task.parent_task_id)) {
        taskMap.get(task.parent_task_id).children.push(task);
      } else {
        roots.push(task);
      }
    });

    const flattened = [];
    function flatten(nodes) {
      nodes.forEach(n => {
        flattened.push(n);
        if (n.children.length > 0) flatten(n.children);
      });
    }
    flatten(roots);

    listEl.innerHTML = flattened.map(task => {
      const priorityClass = `priority-${task.priority}`;
      const isCompleted = task.status === 'done';
      const isSubtask = task.parent_task_id !== null;
      const checked = isCompleted ? 'checked' : '';
      
      return `
        <div class="task-item ${priorityClass} ${isCompleted ? 'completed' : ''} ${isSubtask ? 'is-subtask' : ''} ${task.is_blocked ? 'is-blocked' : ''}" data-id="${task.id}">
          <input type="checkbox" ${checked} class="task-toggle" title="Mark as done" />
          <div class="task-content">
            <div class="task-title">
              ${task.is_blocked ? '<span class="task-blocker-icon" title="Blocked by incomplete tasks">🔒</span>' : ''}
              ${task.title}
            </div>
            <div class="task-meta">
               ${task.project ? `<span class="task-project">#${task.project}</span>` : ""}
               ${task.due_date ? `<span class="task-due">📅 ${task.due_date}</span>` : ""}
               ${task.energy_level ? `<span class="task-energy">⚡ ${task.energy_level.replace('_', ' ')}</span>` : ""}
               <span class="task-badge">${task.priority}</span>
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
      detailRecurrence.value = task.recurrence_rule || "";

      // Hide subtask section if this is already a level-2 subtask
      const subtaskSection = document.getElementById("subtasks-section");
      if (task.parent_task_id) {
        subtaskSection.classList.add("hidden");
      } else {
        subtaskSection.classList.remove("hidden");
        // Load subtasks for this task
        const allTasks = await invoke("task_list");
        const subtasks = allTasks.filter(t => t.parent_task_id === task.id);
        renderSubtaskMiniList(subtasks);
      }

      // Load dependencies
      const deps = await invoke("task_get_dependencies", { taskId: task.id });
      renderDependencyList(deps, task.id);

      detailPanel.classList.remove("hidden");
    } catch (err) {
      console.error("Failed to get task details:", err);
    }
  }

  function renderDependencyList(deps, currentTaskId) {
    const container = document.getElementById("dependency-list");
    container.innerHTML = deps.map(d => `
      <div class="dependency-chip">
        <span>${d.title}</span>
        <span class="remove-dep" data-blocked="${currentTaskId}" data-blocking="${d.id}">×</span>
      </div>
    `).join("");

    container.querySelectorAll(".remove-dep").forEach(btn => {
      btn.addEventListener("click", async () => {
        const { blocked, blocking } = btn.dataset;
        await invoke("task_remove_dependency", { blockedId: blocked, blockingId: blocking });
        const newDeps = await invoke("task_get_dependencies", { taskId: blocked });
        renderDependencyList(newDeps, blocked);
        refreshTasks(searchInput.value);
      });
    });
  }

  function renderSubtaskMiniList(subtasks) {
    const container = document.getElementById("subtask-list");
    if (!subtasks || subtasks.length === 0) {
      container.innerHTML = "";
      return;
    }

    container.innerHTML = subtasks.map(s => `
      <div class="subtask-mini-item">
        <input type="checkbox" ${s.status === 'done' ? 'checked' : ''} disabled />
        <span>${s.title}</span>
      </div>
    `).join("");
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

  document.getElementById("btn-add-subtask").addEventListener("click", async () => {
    try {
      const parentId = detailId.value;
      const id = await invoke("task_create", { 
        title: "New Subtask", 
        parent_task_id: parentId 
      });
      // Refresh both
      refreshTasks(searchInput.value);
      openDetail(parentId);
    } catch (err) {
      // showError will be called if it exceeds limit
      console.error("Failed to create subtask:", err);
      alert(err);
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
      reminder_fired: false,
      time_estimate: null,
      time_logged: 0,
      tags: "[]",
      labels: "[]",
      category: null,
      project: detailProject.value || null,
      notes: null,
      recurrence_rule: detailRecurrence.value || null,
      next_occurrence: null,
      energy_level: detailEnergy.value || null,
      context_tag: null,
      linked_url: null,
      ai_created: false,
      is_blocked: false, // Calculated by backend on read
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

  // --- TAB FILTERING ---
  taskTabs.forEach(tab => {
    tab.addEventListener("click", () => {
      taskTabs.forEach(t => t.classList.remove("active"));
      tab.classList.add("active");
      currentFilter = tab.dataset.filter;
      refreshTasks(searchInput.value);
    });
  });

  // --- DEPENDENCY SEARCH ---
  const depSearchInput = document.getElementById("dependency-search");
  const depSearchResults = document.getElementById("dependency-search-results");

  depSearchInput.addEventListener("input", async (e) => {
    const query = e.target.value.trim();
    if (query.length < 2) {
      depSearchResults.classList.add("hidden");
      return;
    }

    try {
      const results = await invoke("task_search", { query });
      const currentId = detailId.value;
      // Filter out self and already added dependencies? 
      // For now just self.
      const filtered = results.filter(r => r.id !== currentId).slice(0, 5);
      
      if (filtered.length === 0) {
        depSearchResults.innerHTML = '<div class="mini-search-item">No tasks found</div>';
      } else {
        depSearchResults.innerHTML = filtered.map(r => `
          <div class="mini-search-item" data-id="${r.id}">${r.title}</div>
        `).join("");
      }
      depSearchResults.classList.remove("hidden");

      depSearchResults.querySelectorAll(".mini-search-item").forEach(item => {
        item.addEventListener("click", async () => {
          const blockingId = item.dataset.id;
          const blockedId = detailId.value;
          await invoke("task_add_dependency", { blockedId, blockingId });
          
          depSearchInput.value = "";
          depSearchResults.classList.add("hidden");
          
          const newDeps = await invoke("task_get_dependencies", { taskId: blockedId });
          renderDependencyList(newDeps, blockedId);
          refreshTasks(searchInput.value);
        });
      });
    } catch (err) {
      console.error("Dependency search failed:", err);
    }
  });

  // Close search results when clicking outside
  document.addEventListener("click", (e) => {
    if (!depSearchInput.contains(e.target) && !depSearchResults.contains(e.target)) {
      depSearchResults.classList.add("hidden");
    }
  });

  // --- VIEW SWITCHING ---
  viewTabs.forEach(tab => {
    tab.addEventListener("click", () => {
      viewTabs.forEach(t => t.classList.remove("active"));
      tab.classList.add("active");
      currentView = tab.dataset.view;
      refreshTasks(searchInput.value);
    });
  });

  // Initial load
  refreshTasks();
}
