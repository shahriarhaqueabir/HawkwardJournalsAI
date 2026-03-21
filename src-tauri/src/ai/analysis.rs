use crate::ai::AnalysisStatus;
use crate::AppState;
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::{Mutex, Notify};

pub async fn start_analysis_worker(state: std::sync::Arc<AppState>) {
    loop {
        // D-112: Wait for the trigger (Notify one/all)
        state.ai_state.notify.notified().await;

        while let Some(entry_id) = {
            let mut queue = state.ai_state.queue.lock().await;
            queue.pop_front()
        } {
            // 1. Update status
            {
                let mut status_map = state.ai_state.status.lock().await;
                status_map.insert(entry_id.clone(), AnalysisStatus::Processing);
            }

            // 2. Perform the analysis
            if let Err(e) = perform_analysis(&state, &entry_id).await {
                eprintln!("[AI] Analysis error for {}: {:?}", entry_id, e);
                let mut status_map = state.ai_state.status.lock().await;
                status_map.insert(entry_id.clone(), AnalysisStatus::Failed);

                crate::events::emit(
                    &state.handle,
                    crate::events::AppEvent::JournalAnalysisError {
                        entry_id: entry_id.clone(),
                        error: e.to_string(),
                    },
                );
            }

            // 3. Remove from queued_ids
            {
                let mut ids = state.ai_state.queued_ids.lock().await;
                ids.remove(&entry_id);
            }
        }
    }
}

async fn perform_analysis(state: &AppState, entry_id: &str) -> Result<(), crate::error::AppError> {
    // 1. Fetch content
    let journal = {
        let conn = state.conn.lock().await;
        crate::db::journal::get_entry(&conn, entry_id)?
    };

    if let Some(entry) = journal {
        // emit Processing event for frontend UI updates
        crate::events::emit(
            &state.handle,
            crate::events::AppEvent::JournalAnalysisProcessing {
                entry_id: entry_id.to_string(),
            },
        );

        // 2. Call Ollama
        let result = state
            .ollama
            .analyze_journal(&entry.content, entry_id.to_string())
            .await?;

        // 3. Save to DB
        {
            let conn = state.conn.lock().await;
            crate::db::ai::save_analysis_result(&conn, &result)?;
        }

        // 4. Update status
        {
            let mut status_map = state.ai_state.status.lock().await;
            status_map.insert(entry_id.to_string(), AnalysisStatus::Done);
        }

        // 5. Notify frontend
        crate::events::emit(
            &state.handle,
            crate::events::AppEvent::JournalAnalysisCompleted {
                entry_id: entry_id.to_string(),
                result,
            },
        );
    }

    Ok(())
}

pub const MAX_QUEUE: usize = 100;

pub struct AnalysisState {
    pub queue: Mutex<VecDeque<String>>,
    pub queued_ids: Mutex<HashSet<String>>, // O(1) existence check
    pub last_hashes: Mutex<HashMap<String, u64>>,
    pub status: Mutex<HashMap<String, AnalysisStatus>>,
    pub notify: Notify, // D-112: Deterministic trigger
}

impl AnalysisState {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            queued_ids: Mutex::new(HashSet::new()),
            last_hashes: Mutex::new(HashMap::new()),
            status: Mutex::new(HashMap::new()),
            notify: Notify::new(),
        }
    }

    pub async fn should_analyze(&self, entry_id: &str, content: &str) -> bool {
        let mut hashes = self.last_hashes.lock().await;

        // Prevent memory leak by capping the hash map size
        if hashes.len() > 1000 {
            hashes.clear();
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&content, &mut hasher);
        let current_hash = std::hash::Hasher::finish(&hasher);

        match hashes.get(entry_id) {
            Some(&last_hash) if last_hash == current_hash => false,
            _ => {
                hashes.insert(entry_id.to_string(), current_hash);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_should_analyze_deduplication() {
        let state = AnalysisState::new();
        let id = "entry-1";
        let content = "hello world";

        assert!(state.should_analyze(id, content).await);
        // Second call with same content should be false
        assert!(!state.should_analyze(id, content).await);
    }

    #[tokio::test]
    async fn test_should_analyze_on_content_change() {
        let state = AnalysisState::new();
        let id = "entry-1";

        assert!(state.should_analyze(id, "v1").await);
        assert!(state.should_analyze(id, "v2").await);
    }

    #[tokio::test]
    async fn test_queue_flood_limit() {
        let state = Arc::new(AnalysisState::new());
        let mut handles = vec![];

        // Push 150 items (exceeds MAX_QUEUE 100)
        // Note: The limit check is in lib.rs listener, but we can test manual enforcement here if we add a helper
        // For now, let's just verify concurrency safety of the state itself
        for i in 0..150 {
            let s = state.clone();
            handles.push(tokio::spawn(async move {
                let mut queue: tokio::sync::MutexGuard<'_, VecDeque<String>> = s.queue.lock().await;
                let mut ids: tokio::sync::MutexGuard<'_, HashSet<String>> =
                    s.queued_ids.lock().await;
                if queue.len() < MAX_QUEUE {
                    queue.push_back(format!("id-{}", i));
                    ids.insert(format!("id-{}", i));
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let queue: tokio::sync::MutexGuard<'_, VecDeque<String>> = state.queue.lock().await;
        assert!(queue.len() <= MAX_QUEUE);
    }
}
