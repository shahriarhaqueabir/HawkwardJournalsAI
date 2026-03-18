# 🎨 HawkwardJournalAI — Offline-First Desktop Productivity

**HawkwardJournalAI** is a private, offline-first Windows desktop application built with **Tauri v2**. It seamlessly combines a plain-text journal, a full task manager, and a local AI assistant powered by **Ollama**, allowing for secure, local processing of your personal data.

---

## 🚀 Key Features

- **Private by Design**: All data stays on your machine in a single SQLite file.
- **Journal-to-Task Pipeline**: Background AI analysis extracts actionable tasks from your journal entries.
- **Local AI**: Powered by **Ollama (llama3.2)**—no internet required, no subscriptions.
- **Task Management**: Full kanban/list views with subtasks and recurring reminders.
- **Offline-First**: Zero dependencies on cloud services or external APIs (except local Ollama).

---

## 🛠️ Tech Stack

- **Backend**: Rust (Stable 1.77+, MSVC toolchain)
- **Frontend**: Vanilla HTML + CSS + JS (Zero bundlers, zero frameworks)
- **Desktop Shell**: Tauri v2
- **Database**: SQLite (via `rusqlite` bundled-full)
- **AI Engine**: Ollama (llama3.2) via REST API

---

## 📦 Getting Started

### Prerequisites

1.  **Rust**: Install via [rustup.rs](https://rustup.rs/).
2.  **Ollama**: Download from [ollama.com](https://ollama.com/).
3.  **Tauri Dependencies**: Follow the [Tauri v2 Windows setup guide](https://v2.tauri.app/start/prerequisites/).

### Initializing the AI

Before running the app, ensure you have the required AI model installed:

```bash
ollama pull llama3.2
```

### Running the App

```bash
# Clone the repository
git clone https://github.com/shahr/HawkwardJournalAI.git
cd HawkwardJournalAI

# Run in development mode
cargo tauri dev
```

---

## 🏗️ Project Structure

- `src-tauri/`: Rust backend, database migrations, and AI orchestrator.
- `src/`: Vanilla JS frontend, layout grid, and UI components.
- `AgentDocs/`: Detailed system specifications and architectural blueprints.
- `data/`: Local database storage (created on first run).

---

## 🔒 Security & Privacy

- **No Cloud**: There is no "account" and no "sync." Your data is yours.
- **Local Analytics**: AI processing happens entirely on your CPU/GPU via Ollama.
- **Audit Logs**: All database mutations are tracked locally for your review.

---

## 📜 License

MIT License - Copyright (c) 2026.
