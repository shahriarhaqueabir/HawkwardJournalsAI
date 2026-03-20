import { invoke } from "../ipc.js";

let currentConversationId = null;
let isTyping = false;

export async function initAiChat() {
    const btnSend = document.getElementById("btn-ai-send");
    const input = document.getElementById("ai-tab-input");
    const btnNew = document.getElementById("btn-new-chat");

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

    // Initial load
    await loadConversations();

    // Register global event handler for AI Chat
    globalThis.__AI_CHAT_HANDLER__ = handleAiEvent;
}

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
                <div class="conv-title">${c.title || 'Untitled Chat'}</div>
                <div class="conv-meta">${new Date(c.updated_at).toLocaleDateString()}</div>
            </div>
        `).join("");

        list.querySelectorAll(".conversation-item").forEach(item => {
            item.addEventListener("click", () => selectConversation(item.dataset.id));
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
    let content = m.content;

    // Basic markdown support for AI responses if marked is available
    if (m.role === 'assistant' && window.marked) {
        content = window.marked.parse(content);
    }

    return `
        <div class="message-row ${m.role}">
            <div class="message-bubble ${roleClass}">
                ${content}
            </div>
        </div>
    `;
}

async function sendMessage() {
    const input = document.getElementById("ai-tab-input");
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
    
    if (done) {
        // Parse markdown if available
        if (window.marked) {
            bubble.innerHTML = window.marked.parse(bubble.textContent);
        }
    }
    
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

    if (card) {
        card.innerHTML = `
            <div class="tool-header">
                <strong>Tool Executed: ${payload.name}</strong>
            </div>
            <div class="tool-body">
                <em>Action completed successfully. AI is interpreting the results...</em>
            </div>
        `;
    }
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
            source: "journal_entry",
            entryId: entryId
        });
        
        await loadConversations();
        await selectConversation(currentConversationId);
    } catch (e) {
        console.error("Failed to jump to AI chat:", e);
    }
};
