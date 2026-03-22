import { invoke } from "./ipc.js";

export function initAiSidebar() {
  const companionEl = document.getElementById("ai-sidebar-companion");
  const messagesEl = document.querySelector("#right-sidebar .ai-chat-messages");
  const currentEntryId = document.getElementById("current-entry-id");
  const input = document.getElementById("ai-chat-input");
  const sendBtn = document.getElementById("btn-ai-sidebar-send");

  let sidebarConversationId = null;
  let lastEntryIdForConversation = null;

  if (!messagesEl) return;

  // Single typed event handler (Called by app.js dispatcher)
  globalThis.__AI_SIDEBAR_HANDLER__ = (payload) => {
    const { type } = payload;

    switch (type) {
      case "ai_tool_pending":
        showProcessingStatus(`Requesting confirmation for ${payload.name}...`);
        renderToolConfirmation(payload);
        break;
      case "ai_tool_result":
        showProcessingStatus(`Interpreting results from ${payload.name}...`);
        updateToolCard(payload);
        break;
      case "ai_confirm_timeout":
        updateToolTimeoutCard(payload);
        break;
      case "ai_status":
        if (input === document.activeElement || payload.message?.includes("Searching")) {
          showProcessingStatus(payload.message || "Thinking...");
        }
        break;

      case "ai_token":
        appendSidebarToken(payload.token, payload.done);
        break;

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

      case "ai_proactive_nudge":
        renderProactiveNudge(payload.content, payload.trigger);
        break;

      case "ai_reflection_prompt":
        renderReflectionPrompt(payload.content, payload.suggested_tags || []);
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

  function showProcessingStatus(label = "Thinking deeply about your entry...") {
    messagesEl.innerHTML = `
      <div class="ai-bubble processing">
        <div class="spinner"></div>
        ${label}
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

  function appendSidebarToken(token, done) {
    let row = messagesEl.querySelector(".message-row.assistant:last-child");
    if (!row) {
      messagesEl.insertAdjacentHTML(
        "beforeend",
        `<div class="message-row assistant"><div class="message-bubble ai-bubble"></div></div>`
      );
      row = messagesEl.querySelector(".message-row.assistant:last-child");
    }

    const bubble = row.querySelector(".ai-bubble");
    bubble.textContent += token;
    if (done) {
      bubble.classList.remove("processing");
    }
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  function renderProactiveNudge(content, trigger) {
    if (!companionEl || !content) return;

    const label = trigger === "empty_entry" ? "Companion Nudge" : "Check-In";
    const existingReflection = companionEl.querySelector('[data-card="reflection-prompt"]');
    const reflectionHtml = existingReflection ? existingReflection.outerHTML : "";
    companionEl.innerHTML = `
      <div class="companion-card" data-card="proactive-nudge">
        <div class="companion-card-header">
          <div class="companion-card-title">${label}</div>
          <button class="btn-ghost btn-sm" data-dismiss-card="proactive-nudge" type="button">Dismiss</button>
        </div>
        <div class="companion-card-body">${escapeHtml(content)}</div>
      </div>
      ${reflectionHtml}
    `;
    bindCardDismiss();
    bindTryAnother();
  }

  function renderReflectionPrompt(content) {
    if (!companionEl || !content) return;

    const existingNudge = companionEl.querySelector('[data-card="proactive-nudge"]');
    const nudgeHtml = existingNudge ? existingNudge.outerHTML : "";
    companionEl.innerHTML = `
      ${nudgeHtml}
      <div class="companion-card" data-card="reflection-prompt">
        <div class="companion-card-header">
          <div class="companion-card-title">Reflection Prompt</div>
          <button class="btn-ghost btn-sm" data-dismiss-card="reflection-prompt" type="button">Dismiss</button>
        </div>
        <div class="companion-card-body">${escapeHtml(content)}</div>
        <div class="companion-card-actions">
          <button class="btn-primary btn-sm" id="btn-reflection-try-another" type="button">Try Another</button>
        </div>
      </div>
    `;

    bindCardDismiss();
    bindTryAnother();
  }

  function bindTryAnother() {
    companionEl?.querySelector("#btn-reflection-try-another")?.addEventListener("click", async () => {
      await requestReflectionPrompt({ tryAnother: true });
    });
  }

  function bindCardDismiss() {
    companionEl?.querySelectorAll("[data-dismiss-card]").forEach((button) => {
      if (button.dataset.listened) return;
      button.dataset.listened = "true";
      button.addEventListener("click", () => {
        const card = companionEl.querySelector(`[data-card="${button.dataset.dismissCard}"]`);
        card?.remove();
      });
    });
  }

  async function requestProactiveNudge(trigger) {
    try {
      await invoke("ai_maybe_emit_proactive_nudge", { trigger }, { silent: true });
    } catch (err) {
      console.warn("Proactive nudge request failed:", err);
    }
  }

  async function requestReflectionPrompt({ tryAnother = false } = {}) {
    const title = document.getElementById("journal-title")?.value || "";
    const bodySoFar = document.getElementById("journal-editor")?.value || "";

    try {
      const response = await invoke(
        "ai_generate_reflection_prompt",
        {
          title,
          bodySoFar,
          tryAnother,
        },
        { silent: true }
      );

      if (response?.content) {
        renderReflectionPrompt(response.content, response.suggested_tags || []);
      }
    } catch (err) {
      console.warn("Reflection prompt request failed:", err);
    }
  }

  globalThis.requestSidebarProactiveNudge = requestProactiveNudge;
  globalThis.requestSidebarReflectionPrompt = requestReflectionPrompt;

  async function sendSidebarMessage() {
    const text = input?.value?.trim();
    if (!text) return;

    const entryId = currentEntryId?.value || null;
    if (entryId !== lastEntryIdForConversation) {
      sidebarConversationId = null;
      lastEntryIdForConversation = entryId;
    }
    messagesEl.insertAdjacentHTML(
      "beforeend",
      `<div class="message-row user"><div class="message-bubble user-bubble"></div></div>`
    );
    const bubble = messagesEl.querySelector(".message-row.user:last-child .user-bubble");
    bubble.textContent = text;
    messagesEl.scrollTop = messagesEl.scrollHeight;
    input.value = "";

    try {
      sidebarConversationId = await invoke("ai_chat", {
        conversationId: sidebarConversationId,
        message: text,
        source: "sidebar",
        entryId,
      });
    } catch (err) {
      showErrorStatus(String(err));
    }
  }

  sendBtn?.addEventListener("click", sendSidebarMessage);
  input?.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendSidebarMessage();
    }
  });

  function renderToolConfirmation(payload) {
    if (!messagesEl) return;
    const html = `
        <div class="tool-card message-row system" id="tool-${payload.call_id}">
            <div class="tool-header">
                <strong>AI Action: ${payload.name.replace("_", " ")}</strong>
            </div>
            <div class="tool-body">
                ${payload.description}
                <pre style="max-height: 100px; overflow-y: auto; font-size: 11px;">${JSON.stringify(payload.args, null, 2)}</pre>
            </div>
            <div class="tool-actions">
                <button class="btn-primary btn-sm btn-confirm" onclick="confirmAiTool('${payload.call_id}', true)">Confirm</button>
                <button class="btn-ghost btn-sm btn-cancel" onclick="confirmAiTool('${payload.call_id}', false)">Cancel</button>
            </div>
        </div>
    `;
    messagesEl.insertAdjacentHTML("beforeend", html);
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  function updateToolCard(payload) {
    let card = document.getElementById(`tool-${payload.call_id}`);
    if (!card) return;

    const status = payload.result?.status;
    const isError = status === "error";
    const isCancelled = status === "cancelled" || payload.confirmed === false;
    const heading = isError
      ? `Failed: ${payload.name.replace("_", " ")}`
      : isCancelled
        ? `Cancelled: ${payload.name.replace("_", " ")}`
        : `Completed: ${payload.name.replace("_", " ")}`;

    const message = isError
      ? escapeHtml(payload.result?.message || "Error")
      : isCancelled
        ? escapeHtml(payload.result?.message || "Cancelled")
        : "Done. AI is interpreting...";

    card.innerHTML = `
        <div class="tool-header">
            <strong>${heading}</strong>
        </div>
        <div class="tool-body">
            <em>${message}</em>
        </div>
    `;
  }

  function updateToolTimeoutCard(payload) {
    const card = document.getElementById(`tool-${payload.call_id}`);
    if (!card) return;

    card.innerHTML = `
        <div class="tool-header">
            <strong>Timed out: ${payload.tool_name.replace("_", " ")}</strong>
        </div>
        <div class="tool-body">
            <em>Request expired after 300 seconds.</em>
        </div>
    `;
  }

  requestProactiveNudge("app_open");
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text ?? "";
  return div.innerHTML;
}
