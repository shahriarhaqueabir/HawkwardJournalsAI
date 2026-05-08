import { invoke } from "../ipc.js";

export function initJournal() {
  const listEl = document.getElementById("journal-entry-list");
  const editor = document.getElementById("journal-editor");
  const titleInput = document.getElementById("journal-title");
  const idInput = document.getElementById("current-entry-id");
  const statusEl = document.getElementById("save-status");
  const wordCountEl = document.getElementById("word-count");
  const btnNew = document.getElementById("btn-new-entry");
  const btnDelete = document.getElementById("btn-delete-entry");

  if (!editor || !titleInput || !idInput || !statusEl || !listEl) {
    console.error("Journal UI elements missing.");
    return;
  }

  // State for Keyset Pagination (D-104)
  let entries = [];
  let lastCursor = null;
  let isLoading = false;
  let hasMore = true;
  const PAGE_SIZE = 30;

  // State for Autosave
  let saveTimeout = null;
  let isSaving = false;
  let pendingSave = false;
  let lastSaved = { id: "", title: "", content: "" };

  // ── REFRESH / LOAD LIST ──────────────────────────
  async function refreshList() {
    entries = [];
    lastCursor = null;
    hasMore = true;
    listEl.innerHTML = "";
    await loadMore();
  }

  async function loadMore() {
    if (isLoading || !hasMore) return;
    
    isLoading = true;
    try {
      const result = await invoke("journal_list", { 
        cursor: lastCursor, 
        limit: PAGE_SIZE 
      });

      if (result.length < PAGE_SIZE) {
        hasMore = false;
      }

      if (result.length > 0) {
        lastCursor = result[result.length - 1].created_at;
        appendToList(result);
      } else if (entries.length === 0) {
        listEl.innerHTML = '<div class="entry-list-empty">No entries yet.</div>';
      }
    } catch (err) {
      console.error("Failed to load journal list:", err);
    } finally {
      isLoading = false;
    }
  }

  function appendToList(newEntries) {
    const fragment = document.createDocumentFragment();
    
    newEntries.forEach((entry) => {
      // Avoid duplicates if any (though keyset should prevent this)
      if (entries.some(e => e.id === entry.id)) return;
      entries.push(entry);

      const item = document.createElement("div");
      item.className = `entry-item ${idInput.value === entry.id ? "active" : ""}`;
      item.dataset.id = entry.id;

      const title = entry.title || "Untitled Entry";
      const date = new Date(entry.created_at).toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
      });
      const titleClass = entry.title ? "entry-title" : "entry-title untitled";

      item.innerHTML = `
        <div class="${titleClass}">${title}</div>
        <div class="entry-meta-row">
          <span class="entry-date">${date}</span>
          <span class="entry-stats">${entry.word_count} words</span>
        </div>
      `;

      item.addEventListener("click", () => loadEntry(entry.id));
      fragment.appendChild(item);
    });

    listEl.appendChild(fragment);
  }

  // ── LOAD ENTRY ──────────────────────────────────
  async function loadEntry(id) {
    if (isSaving) {
      setTimeout(() => loadEntry(id), 200);
      return;
    }

    try {
      const entry = await invoke("journal_get", { id });
      if (entry) {
        idInput.value = entry.id;
        titleInput.value = entry.title || "";
        editor.value = entry.content || "";
        updateWordCount();
        lastSaved = { id: entry.id, title: entry.title || "", content: entry.content || "" };
        statusEl.textContent = "Loaded";
        
        // Highlight in list
        listEl.querySelectorAll(".entry-item").forEach(i => i.classList.remove("active"));
        listEl.querySelector(`[data-id="${id}"]`)?.classList.add("active");
      }
    } catch (err) {
      console.error("Failed to load entry:", err);
    }
  }

  // ── NEW ENTRY ───────────────────────────────────
  function createNewEntry() {
    if (isSaving) return;
    idInput.value = "";
    titleInput.value = "";
    editor.value = "";
    updateWordCount();
    lastSaved = { id: "", title: "", content: "" };
    statusEl.textContent = "Ready";
    listEl.querySelectorAll(".entry-item").forEach(i => i.classList.remove("active"));
    editor.focus();
  }

  // ── DELETE ENTRY ─────────────────────────────────
  async function deleteEntry() {
    const id = idInput.value;
    if (!id) return;

    if (!confirm("Are you sure you want to move this entry to trash?")) return;

    try {
      const success = await invoke("journal_delete", { id });
      if (success) {
        createNewEntry();
        refreshList();
      }
    } catch (err) {
      console.error("Delete failed:", err);
    }
  }

  // ── AUTOSAVE ────────────────────────────────────
  function getDraft() {
    return {
      id: idInput.value ?? "",
      title: titleInput.value ?? "",
      content: editor.value ?? "",
    };
  }

  async function save() {
    if (isSaving) {
      pendingSave = true;
      return;
    }

    const { id, title, content } = getDraft();
    const isBlank = !content.trim() && !title.trim();

    if (!id && isBlank) {
      statusEl.textContent = "Ready";
      return;
    }

    if (id === lastSaved.id && title === lastSaved.title && content === lastSaved.content) {
      statusEl.textContent = "Saved";
      return;
    }

    statusEl.textContent = "Saving...";
    isSaving = true;
    pendingSave = false;

    try {
      const isNew = !id;
      const newId = await invoke("save_journal_entry", { id, title, content });
      idInput.value = newId;
      lastSaved = { id: newId, title, content };
      statusEl.textContent = "Saved at " + new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      
      if (isNew) {
        // Re-init the list to show the newest at top
        await refreshList();
      } else {
        // Update the item in the list if it exists
        const item = listEl.querySelector(`[data-id="${newId}"]`);
        if (item) {
          const titleEl = item.querySelector(".entry-title");
          titleEl.textContent = title || "Untitled Entry";
          titleEl.className = title ? "entry-title" : "entry-title untitled";
          const wordCount = content.trim() ? content.split(/\s+/).length : 0;
          item.querySelector(".entry-stats").textContent = `${wordCount} words`;
        }
      }
    } catch (err) {
      console.error("Save Error:", err);
      statusEl.textContent = "Save error";
    } finally {
      isSaving = false;
      if (pendingSave) {
        pendingSave = false;
        await save();
      }
    }
  }

  function updateWordCount() {
    const text = editor.value.trim();
    const count = text ? text.split(/\s+/).length : 0;
    wordCountEl.textContent = `${count} words`;
  }

  let lastEmitTime = 0;

  const triggerAutoSave = () => {
    statusEl.textContent = "Typing...";
    updateWordCount();
    
    // Step 4: Frontend Throttle - 500ms
    const now = Date.now();
    if (now - lastEmitTime > 500) {
      const id = idInput.value;
      if (id) {
        // D-106: Fires immediately
        globalThis.__TAURI__.event.emit('journal_analysis_queued', { id });
        lastEmitTime = now;
      }
    }

    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(save, 1500);
  };

  // ── SCROLL TO LOAD (VirtualList Lite) ────────────────
  listEl.addEventListener("scroll", () => {
    const { scrollTop, scrollHeight, clientHeight } = listEl;
    if (scrollTop + clientHeight >= scrollHeight - 20) {
      loadMore();
    }
  });

  // ── INIT ────────────────────────────────────────
  editor.addEventListener("input", triggerAutoSave);
  titleInput.addEventListener("input", triggerAutoSave);
  btnNew?.addEventListener("click", createNewEntry);
  btnDelete?.addEventListener("click", deleteEntry);

  refreshList();
}
