import { invoke } from "./ipc.js";

let messagesEl = null;
let inputEl = null;
let currentEntryId = null;
let currentConversationId = null;
let currentAiBubble = null;

export function initAiSidebar() {
  messagesEl = document.getElementById("ai-chat-messages");
  inputEl = document.getElementById("ai-chat-input");
  currentEntryId = document.getElementById("current-entry-id");

  if (inputEl) {
    inputEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        sendChatMessage();
      }
    });
  }
}

// ── PUBLIC EVENT HANDLER (D-96) ────────────────────────
export function handleAppEvent(payload) {
  if (!messagesEl || !currentEntryId) return;

  const { type } = payload;

  switch (type) {
    case "journal_analysis_queued":
      if (currentEntryId.value === payload.entry_id) {
        showQueuedStatus();
      }
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

    case "ai_token":
      if (payload.conversation_id === currentConversationId) {
        appendToken(payload.token);
      }
      break;

    case "ai_response_complete":
      if (payload.conversation_id === currentConversationId) {
        currentAiBubble = null;
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
}

function showQueuedStatus() {
  if (!messagesEl) return;
  messagesEl.innerHTML = `
    <div class="ai-bubble queued">
      <div class="pulse"></div>
      Waiting in line for analysis...
    </div>
  `;
}

function showProcessingStatus() {
  if (!messagesEl) return;
  messagesEl.innerHTML = `
    <div class="ai-bubble processing">
      <div class="spinner"></div>
      Thinking deeply about your entry...
    </div>
  `;
}

function showErrorStatus(error) {
  if (!messagesEl) return;
  messagesEl.innerHTML = `
    <div class="ai-bubble error">
      <strong>Insight Error</strong><br>
      ${error}
    </div>
  `;
}

function renderAnalysisResult(result) {
  if (!messagesEl) return;
  const { summary, emotions, tasks, insights, facts, mood } = result;

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

  if (facts && facts.length > 0) {
    html += `
      <div class="analysis-section facts">
        <h3>Proposed Facts</h3>
        <ul class="fact-suggestions">
          ${facts.map(f => `
            <li>
              <div class="fact-content">
                <span class="fact-category badge">${f.category}</span>
                <span class="fact-text">${f.content}</span>
              </div>
              <div class="fact-actions">
                <button class="btn-fact-add btn-ghost-sm" data-key="${f.key}" data-content="${f.content}" data-category="${f.category}">Accept</button>
                <button class="btn-fact-reject btn-ghost-sm">Reject</button>
              </div>
            </li>
          `).join("")}
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

  // Fact listeners (Phase 3 wiring)
  messagesEl.querySelectorAll(".btn-fact-add").forEach(btn => {
    btn.addEventListener("click", async () => {
      const now = new Date().toISOString();
      const fact = {
        id: crypto.randomUUID(),
        fact_key: btn.dataset.key,
        content: btn.dataset.content,
        category: btn.dataset.category,
        confidence: 0.8, // Default confidence for user-accepted facts
        source_entry_id: currentEntryId.value || null,
        created_at: now,
        updated_at: now
      };

      try {
        btn.disabled = true;
        btn.textContent = "...";
        await invoke("profile_upsert_fact", { fact });
        btn.textContent = "✓ Accepted";
        btn.classList.add("added");
        const rejectBtn = btn.parentElement.querySelector(".btn-fact-reject");
        if (rejectBtn) rejectBtn.remove();
      } catch (err) {
        btn.textContent = "Error";
        btn.disabled = false;
        console.error("Failed to add fact:", err);
      }
    });
  });

  messagesEl.querySelectorAll(".btn-fact-reject").forEach(btn => {
    btn.addEventListener("click", () => {
      const li = btn.closest("li");
      const acceptBtn = li.querySelector(".btn-fact-add");
      if (acceptBtn) acceptBtn.remove();
      btn.textContent = "✗ Rejected";
      btn.disabled = true;
      li.style.opacity = "0.5";
    });
  });
}

async function sendChatMessage() {
  const text = inputEl.value.trim();
  if (!text) return;

  inputEl.value = "";
  appendUserMessage(text);

  try {
    currentConversationId = await invoke("ai_chat", {
      conversationId: currentConversationId,
      message: text
    });
    
    currentAiBubble = createAiBubble();
  } catch (err) {
    console.error("AI Chat Error:", err);
    const bubble = createAiBubble();
    bubble.classList.add("error");
    bubble.textContent = "I'm having trouble thinking right now. Is Ollama running?";
  }
}

function appendUserMessage(text) {
  const div = document.createElement("div");
  div.className = "chat-bubble user";
  div.textContent = text;
  messagesEl.appendChild(div);
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

function createAiBubble() {
  const div = document.createElement("div");
  div.className = "chat-bubble ai";
  messagesEl.appendChild(div);
  messagesEl.scrollTop = messagesEl.scrollHeight;
  return div;
}

function appendToken(token) {
  if (!currentAiBubble) {
    currentAiBubble = createAiBubble();
  }
  currentAiBubble.textContent += token;
  messagesEl.scrollTop = messagesEl.scrollHeight;
}
