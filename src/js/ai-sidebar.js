import { invoke } from "./ipc.js";

export function initAiSidebar() {
  const messagesEl = document.querySelector(".right-sidebar .ai-chat-messages");
  const currentEntryId = document.getElementById("current-entry-id");

  if (!messagesEl) return;

  // Single typed event handler (Called by app.js dispatcher)
  globalThis.__AI_SIDEBAR_HANDLER__ = (payload) => {
    const { type } = payload;

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
  };

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
                <span>${t.title}</span>
                <button class="btn-task-add" data-task="${t.title}" data-project="${t.project_id}">Add</button>
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

    html += `
        <div class="analysis-footer">
          <button id="btn-ai-followup" class="btn-primary btn-sm" style="margin-top: 20px; width: 100%;">Ask AI about this entry...</button>
        </div>
      </div>
    `;
    messagesEl.innerHTML = html;

    // Follow-up listener
    const btnFollowup = messagesEl.querySelector("#btn-ai-followup");
    if (btnFollowup) {
      btnFollowup.addEventListener("click", () => {
        if (globalThis.jumpToAiChat) {
          globalThis.jumpToAiChat(currentEntryId.value);
        }
      });
    }

    // Task listeners
    messagesEl.querySelectorAll(".btn-task-add").forEach(btn => {
      btn.addEventListener("click", async () => {
        const title = btn.dataset.task;
        const projectId = btn.dataset.project || "inbox";
        try {
          btn.disabled = true;
          btn.textContent = "...";
          await invoke("task_create", { title, projectId });
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
