import { invoke } from "./ipc.js";

export function initAiSidebar() {
  const messagesEl = document.getElementById("ai-chat-messages");
  const currentEntryId = document.getElementById("current-entry-id");

  if (!messagesEl) return;

  // SINGLE typed event listener (D-96)
  globalThis.__TAURI__.event.listen("app_event", (event) => {
    const payload = event.payload;
    const { type } = payload;

    // Dispatch based on type
    switch (type) {
      case "journal_analysis_processing":
        if (currentEntryId.value === payload.entry_id) {
          showProcessingStatus();
        }
        break;

      case "journal_analysis_completed":
        if (currentEntryId.value === payload.entry_id) {
          renderAnalysisResult(payload.result);
        }
        break;

      case "journal_analysis_error":
        if (currentEntryId.value === payload.entry_id) {
          showErrorStatus(payload.error);
        }
        break;

      case "ai_model_missing":
        showErrorStatus(`
          <strong>Model Not Found</strong><br>
          The AI model <code>${payload.model}</code> is not installed.<br>
          Please run <code>ollama pull ${payload.model}</code> in your terminal.
        `);
        break;
    }
  });

  function showProcessingStatus() {
    messagesEl.innerHTML = `
      <div class="ai-bubble processing">
        <div class="spinner"></div>
        Thinking deeply about your entry...
      </div>
    `;
  }

  function showErrorStatus(error) {
    messagesEl.innerHTML = `
      <div class="ai-bubble error">
        <strong>Insight Error</strong><br>
        ${error}
      </div>
    `;
  }

  function renderAnalysisResult(result) {
    const { summary, emotions, tasks, insights, mood } = result;

    let html = `
      <div class="analysis-card">
        <div class="analysis-section summary">
          <div class="mood-badge">${mood || "Reflective"}</div>
          <h3>Key Summary</h3>
          <p>${summary}</p>
        </div>
    `;

    if (emotions && emotions.length > 0) {
      html += `
        <div class="analysis-section emotions">
          <h3>Emotions</h3>
          <div class="tag-cloud">
            ${emotions.map(e => `<span class="tag emotion">${e}</span>`).join("")}
          </div>
        </div>
      `;
    }

    if (tasks && tasks.length > 0) {
      html += `
        <div class="analysis-section tasks">
          <h3>Action Items</h3>
          <ul class="task-suggestions">
            ${tasks.map(t => `
              <li>
                <span>${t}</span>
                <button class="btn-task-add" data-task="${t}">Add</button>
              </li>
            `).join("")}
          </ul>
        </div>
      `;
    }

    if (insights && insights.length > 0) {
      html += `
        <div class="analysis-section insights">
          <h3>Deeper Insights</h3>
          <ul>
            ${insights.map(i => `<li>${i}</li>`).join("")}
          </ul>
        </div>
      `;
    }

    html += `</div>`;
    messagesEl.innerHTML = html;

    // Task listeners (Phase 2 wiring)
    messagesEl.querySelectorAll(".btn-task-add").forEach(btn => {
      btn.addEventListener("click", async () => {
        const title = btn.dataset.task;
        try {
          btn.disabled = true;
          btn.textContent = "...";
          await invoke("task_create", { title });
          btn.textContent = "✓ Added";
          btn.classList.add("added");
        } catch (err) {
          btn.textContent = "Error";
          btn.disabled = false;
        }
      });
    });
  }
}
