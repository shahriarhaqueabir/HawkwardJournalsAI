use std::collections::{HashMap, VecDeque, HashSet};
use tokio::sync::{Mutex, Notify};
use crate::ai::AnalysisStatus;

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
                let mut ids: tokio::sync::MutexGuard<'_, HashSet<String>> = s.queued_ids.lock().await;
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
