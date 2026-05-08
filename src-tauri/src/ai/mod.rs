pub mod analysis;
pub mod client;
pub mod fallback;
pub mod keywords;
pub mod prompt;
pub mod stream;
pub mod tools;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AnalysisStatus {
    Queued,
    Processing,
    Done,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct RawAnalysis {
    pub summary: String,
    pub mood: String,
    pub emotions: Option<Vec<String>>,
    pub tasks: Option<Vec<String>>,
    pub insights: Option<Vec<String>>,
    pub triplets: Option<Vec<(String, String, String)>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisResult {
    pub id: String,
    pub summary: String,
    pub mood: String,
    pub emotions: Vec<String>,
    pub tasks: Vec<String>,
    pub insights: Vec<String>,
    pub triplets: Vec<(String, String, String)>,
}

impl AnalysisResult {
    pub fn from_raw(raw: RawAnalysis, id: String) -> Self {
        Self {
            id,
            summary: clean(raw.summary),
            mood: clean(raw.mood),
            emotions: raw.emotions.unwrap_or_default(),
            tasks: raw.tasks.unwrap_or_default(),
            insights: raw.insights.unwrap_or_default(),
            triplets: raw.triplets.unwrap_or_default(),
        }
    }
}

fn clean(s: String) -> String {
    s.trim().replace('\n', " ").chars().take(200).collect()
}
