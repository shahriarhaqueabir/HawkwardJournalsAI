import { invoke } from "../ipc.js";

let currentConversationId = null;
let isTyping = false;

export async function initAiChat() {
    const btnSend = document.getElementById("btn-ai-send");
    const input = document.getElementById("ai-tab-input");
    const btnNew = document.getElementById("btn-new-chat");
    const btnAddPinned = document.getElementById("btn-add-pinned-fact");

    if (btnSend) {
        btnSend.addEventListener("click", sendMessage);
    }

    if (input) {
        input.addEventListener("keydown", (e) => {
            if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                sendMessage();
            }
        });
    }

    if (btnNew) {
        btnNew.addEventListener("click", startNewChat);
    }

    if (btnAddPinned) {
        btnAddPinned.addEventListener("click", promptAddPinnedFact);
    }

    // Initial load
    await loadConversations();
    await loadPinnedFacts();

    // Register global event handler for AI Chat
    globalThis.__AI_CHAT_HANDLER__ = handleAiEvent;
}

async function loadPinnedFacts() {
    const list = document.getElementById("ai-pinned-fact-list");
    if (!list) return;

    try {
        const facts = await invoke("ai_list_pinned_memory");
        renderPinnedFacts(facts);
    } catch (e) {
        console.error("Failed to load pinned facts:", e);
    }
}

function renderPinnedFacts(facts) {
    const list = document.getElementById("ai-pinned-fact-list");
    if (facts.length === 0) {
        list.innerHTML = '<div class="list-empty">No pinned facts</div>';
        return;
    }

    list.innerHTML = facts.map(f => `
        <div class="pinned-fact-item importance-${f.importance || 1}" title="Added: ${new Date(f.created_at).toLocaleString()}">
            <div class="fact-importance"></div>
            <div class="fact-content">${escapeHtml(f.content)}</div>
            <div class="fact-delete" onclick="deletePinnedFact('${f.id}')">×</div>
        </div>
    `).join("");
}

async function promptAddPinnedFact() {
    const content = prompt("Enter a fact for the AI to remember (e.g., 'User prefers dark mode', 'User is a developer'):");
    if (!content) return;
    
    try {
        await invoke("ai_upsert_pinned_memory", { 
            id: null,
            content, 
            importance: 1 
        });
        await loadPinnedFacts();
    } catch (e) {
        console.error("Failed to add pinned fact:", e);
        alert("Failed to add fact: " + e);
    }
}

globalThis.deletePinnedFact = async (id) => {
    if (!confirm("Delete this pinned fact?")) return;
    try {
        await invoke("ai_delete_pinned_memory", { id });
        await loadPinnedFacts();
    } catch (e) {
        console.error("Failed to delete pinned fact:", e);
        alert("Failed to delete fact: " + e);
    }
};

async function loadConversations() {
    const list = document.getElementById("ai-conversation-list");
    if (!list) return;

    try {
        const convs = await invoke("ai_conversation_list", { source: "ai_tab" });
        if (convs.length === 0) {
            list.innerHTML = '<div class="list-empty">No conversations</div>';
            return;
        }

        list.innerHTML = convs.map(c => `
            <div class="conversation-item ${c.id === currentConversationId ? 'active' : ''}" data-id="${c.id}">
                <div class="conversation-main">
                    <div class="conv-title">${escapeHtml(c.title || 'Untitled Chat')}</div>
                    <div class="conv-meta">${new Date(c.updated_at).toLocaleDateString()}</div>
                </div>
                <button class="conv-delete btn-icon btn-inline" data-delete-id="${c.id}" title="Delete conversation">×</button>
            </div>
        `).join("");

        list.querySelectorAll(".conversation-item").forEach(item => {
            item.addEventListener("click", () => selectConversation(item.dataset.id));
        });
        list.querySelectorAll("[data-delete-id]").forEach((button) => {
            button.addEventListener("click", async (event) => {
                event.stopPropagation();
                const id = button.dataset.deleteId;
                if (!confirm("Delete this conversation?")) return;
                await invoke("ai_conversation_delete", { id });
                if (currentConversationId === id) {
                    startNewChat();
                }
                await loadConversations();
            });
        });
    } catch (e) {
        console.error("Failed to load conversations:", e);
    }
}

async function selectConversation(id) {
    currentConversationId = id;
    document.querySelectorAll(".conversation-item").forEach(i => i.classList.toggle("active", i.dataset.id === id));
    
    const messagesEl = document.getElementById("ai-tab-messages");
    messagesEl.innerHTML = '<div class="loading">Loading messages...</div>';

    try {
        const msgs = await invoke("ai_message_list", { conversationId: id });
        renderMessages(msgs);
    } catch (e) {
        messagesEl.innerHTML = `<div class="error">Error loading messages: ${e}</div>`;
    }
}

function renderMessages(msgs) {
    const messagesEl = document.getElementById("ai-tab-messages");
    if (msgs.length === 0) {
        messagesEl.innerHTML = '<div class="ai-welcome"><h2>New Chat</h2><p>Ask anything...</p></div>';
        return;
    }

    messagesEl.innerHTML = msgs.map(m => createMessageBubble(m)).join("");
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function createMessageBubble(m) {
  const roleClass = m.role === 'user' ? 'user-bubble' : 'ai-bubble';
  const content = escapeHtml(m.content);

    return `
        <div class="message-row ${m.role}">
            <div class="message-bubble ${roleClass}">
                ${content}
            </div>
        </div>
    `;
}

async function sendMessage() {
    const input = document.querySelector(".ai-chat-input-field");
    const text = input.value.trim();
    if (!text || isTyping) return;

    appendUserMessage(text);
    input.value = "";
    isTyping = true;

    try {
        const id = await invoke("ai_chat", {
            conversationId: currentConversationId,
            message: text,
            source: "ai_tab"
        });

        if (!currentConversationId) {
            currentConversationId = id;
            await loadConversations();
        }
    } catch (e) {
        console.error("Chat error:", e);
        appendErrorMessage(e);
        isTyping = false;
    }
}

function appendUserMessage(text) {
    const messagesEl = document.getElementById("ai-tab-messages");
    messagesEl.insertAdjacentHTML("beforeend", createMessageBubble({ role: 'user', content: text }));
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function appendErrorMessage(err) {
    const messagesEl = document.getElementById("ai-tab-messages");
    messagesEl.insertAdjacentHTML("beforeend", `<div class="message-row system"><div class="error-bubble">${err}</div></div>`);
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function handleAiEvent(payload) {
    if (payload.type === "ai_token") {
        hideThinking();
        updateLastAiBubble(payload.token, payload.done);
        if (payload.done) isTyping = false;
    } else if (payload.type === "ai_tool_pending") {
        showThinking(`Requesting confirmation for ${payload.name}...`);
        renderToolConfirmation(payload);
    } else if (payload.type === "ai_tool_result") {
        showThinking(`Interpreting results from ${payload.name}...`);
        updateToolCard(payload);
    } else if (payload.type === "ai_confirm_timeout") {
        updateToolTimeoutCard(payload);
        isTyping = false;
    } else if (payload.type === "ai_status") {
        showThinking(payload.message);
    }
}

function showThinking(statusText = "Thinking...") {
    hideThinking();
    const chatFeed = document.getElementById('ai-tab-messages'); // Changed from ai-chat-feed to ai-tab-messages
    const bubble = document.createElement('div');
    bubble.className = 'message-row system'; // Added message-row system
    bubble.id = 'ai-thinking-bubble'; // Changed from ai-thinking-indicator to ai-thinking-bubble
    bubble.innerHTML = `
        <div class="thinking-bubble">
            <div class="thinking-content">
                <span class="thinking-text">${statusText}</span>
                <div class="thinking-dots">
                    <span class="thinking-dot"></span>
                    <span class="thinking-dot"></span>
                    <span class="thinking-dot"></span>
                </div>
            </div>
        </div>
    `;
    chatFeed.appendChild(bubble);
    chatFeed.scrollTop = chatFeed.scrollHeight;
}

function hideThinking() {
    const thinking = document.getElementById("ai-thinking-bubble");
    if (thinking) thinking.remove();
}

function updateLastAiBubble(token, done) {
    const messagesEl = document.getElementById("ai-tab-messages");
    let lastRow = messagesEl.querySelector(".message-row.assistant:last-child");
    
    if (!lastRow) {
        messagesEl.insertAdjacentHTML("beforeend", `
            <div class="message-row assistant">
                <div class="message-bubble ai-bubble"></div>
            </div>
        `);
        lastRow = messagesEl.querySelector(".message-row.assistant:last-child");
    }

    const bubble = lastRow.querySelector(".ai-bubble");
    bubble.textContent += token;
    
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function renderToolConfirmation(payload) {
    const messagesEl = document.getElementById("ai-tab-messages");
    const html = `
        <div class="tool-card message-row system" id="tool-${payload.call_id}">
            <div class="tool-header">
                <strong>AI Action: ${payload.name}</strong>
            </div>
            <div class="tool-body">
                ${payload.description}
                <pre>${JSON.stringify(payload.args, null, 2)}</pre>
            </div>
            <div class="tool-actions">
                <button class="btn-confirm" onclick="confirmAiTool('${payload.call_id}', true)">Confirm</button>
                <button class="btn-cancel" onclick="confirmAiTool('${payload.call_id}', false)">Cancel</button>
            </div>
        </div>
    `;
    messagesEl.insertAdjacentHTML("beforeend", html);
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

globalThis.confirmAiTool = async (callId, confirmed) => {
    const card = document.getElementById(`tool-${callId}`);
    if (card) {
        card.querySelector(".tool-actions").innerHTML = "<em>Processing...</em>";
    }
    await invoke("ai_confirm_tool", { callId, confirmed });
};

function updateToolCard(payload) {
    let card = document.getElementById(`tool-${payload.call_id}`);
    
    if (!card) {
        // For read-only tools that didn't have a "pending" card
        const messagesEl = document.getElementById("ai-tab-messages");
        const html = `
            <div class="tool-card message-row system" id="tool-${payload.call_id}">
                <div class="tool-header">
                    <strong>AI Action: ${payload.name}</strong>
                </div>
                <div class="tool-body">
                    <em>Automatic execution...</em>
                </div>
            </div>
        `;
        messagesEl.insertAdjacentHTML("beforeend", html);
        card = document.getElementById(`tool-${payload.call_id}`);
        messagesEl.scrollTop = messagesEl.scrollHeight;
    }

    const status = payload.result?.status;
    const isError = status === "error";
    const isCancelled = status === "cancelled" || payload.confirmed === false;
    
    let heading = `Tool Executed: ${payload.name}`;
    if (isError) {
        heading = `Tool Failed: ${payload.name}`;
    } else if (isCancelled) {
        heading = `Tool Cancelled: ${payload.name}`;
    }

    let message = "Action completed successfully. AI is interpreting the results...";
    if (isError) {
        message = escapeHtml(payload.result?.message || "The tool could not complete.");
    } else if (isCancelled) {
        message = escapeHtml(payload.result?.message || "The action was cancelled.");
    }

    if (card) {
        let extraHtml = "";

        if (!isError && payload.name === "list_tasks") {
            const tasks = payload.result?.tasks;
            if (Array.isArray(tasks)) {
                const rows = tasks.slice(0, 12).map((t) => {
                    const id = String(t.id || "");
                    const shortId = id ? id.slice(0, 6) : "??????";
                    const title = escapeHtml(String(t.title || "(untitled)"));
                    const statusTxt = escapeHtml(String(t.status || ""));
                    const due = escapeHtml(String(t.due_date || "No Date"));
                    return `<div class="report-list-item"><span>[${shortId}] ${title}</span><span class="status">${statusTxt} · ${due}</span></div>`;
                }).join("");

                extraHtml = `
                    <div class="tool-body" style="margin-top:10px;">
                        <div class="report-list">${rows || "<div class='report-list-item'>No tasks found.</div>"}</div>
                        <div style="margin-top:8px; color: var(--color-muted); font-size: 12px;">
                            Tip: say “cancel [id6]” or “complete [id6]” (example: cancel [a1b2c3]).
                        </div>
                    </div>
                `;
            }
        }

        card.innerHTML = `
            <div class="tool-header">
                <strong>${heading}</strong>
            </div>
            <div class="tool-body">
                <em>${message}</em>
            </div>
            ${extraHtml}
        `;
    }
}

function updateToolTimeoutCard(payload) {
    const card = document.getElementById(`tool-${payload.call_id}`);
    if (!card) return;

    card.innerHTML = `
        <div class="tool-header">
            <strong>Tool Cancelled: ${payload.tool_name}</strong>
        </div>
        <div class="tool-body">
            <em>The confirmation request expired after 300 seconds.</em>
        </div>
    `;
}

function startNewChat() {
    currentConversationId = null;
    document.getElementById("ai-tab-messages").innerHTML = '<div class="ai-welcome"><h2>New Chat</h2><p>Ask anything...</p></div>';
    document.querySelectorAll(".conversation-item").forEach(i => i.classList.remove("active"));
}

globalThis.jumpToAiChat = async (entryId) => {
    // 1. Switch Tab
    const aiNavItem = document.querySelector('.nav-item[data-tab="ai"]');
    if (aiNavItem) aiNavItem.click();

    // 2. Start New Chat contextually
    startNewChat();
    
    // Create new conversation with entry context
    try {
        currentConversationId = await invoke("ai_chat", {
            conversationId: null,
            message: "Analyze the entry I was just looking at and help me plan my next steps.",
            source: "ai_tab",
            entryId: entryId
        });
        
        await loadConversations();
        await selectConversation(currentConversationId);
    } catch (e) {
        console.error("Failed to jump to AI chat:", e);
    }
};

function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text ?? "";
    return div.innerHTML;
}
