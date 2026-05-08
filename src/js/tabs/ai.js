const { invoke } = window.__TAURI__.core;

class AiChat {
    constructor() {
        this.messagesEl = document.getElementById('chat-messages');
        this.inputEl = document.getElementById('chat-input');
        this.sendBtn = document.getElementById('btn-send-chat');
        this.currentConversationId = null;
        this.currentAiBubble = null;
        
        this.init();
    }

    init() {
        this.sendBtn.addEventListener('click', () => this.sendMessage());
        this.inputEl.addEventListener('keypress', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                this.sendMessage();
            }
        });
        
        // Register for app events
        window.addEventListener('app_event', (e) => this.handleAppEvent(e.detail));
    }

    async sendMessage() {
        const text = this.inputEl.value.trim();
        if (!text) return;

        this.inputEl.value = '';
        this.appendMessage('user', text);
        
        try {
            this.currentConversationId = await invoke('ai_chat', {
                conversationId: this.currentConversationId,
                message: text
            });
            
            // Create a placeholder for the AI response
            this.currentAiBubble = this.appendMessage('ai', '...');
            this.currentAiBubble.textContent = ''; // Clear the dots
        } catch (e) {
            console.error('Chat error:', e);
            this.appendMessage('error', 'Failed to connect to AI.');
        }
    }

    appendMessage(role, text) {
        const div = document.createElement('div');
        div.className = `chat-bubble ${role}`;
        div.textContent = text;
        this.messagesEl.appendChild(div);
        this.messagesEl.scrollTop = this.messagesEl.scrollHeight;
        return div;
    }

    handleAppEvent(payload) {
        const { type, conversation_id, token } = payload;
        
        if (conversation_id !== this.currentConversationId) return;

        if (type === 'ai_token') {
            if (this.currentAiBubble) {
                this.currentAiBubble.textContent += token;
                this.messagesEl.scrollTop = this.messagesEl.scrollHeight;
            }
        } else if (type === 'ai_response_complete') {
            this.currentAiBubble = null;
        }
    }
}

// Global instance
window.aiChat = new AiChat();
