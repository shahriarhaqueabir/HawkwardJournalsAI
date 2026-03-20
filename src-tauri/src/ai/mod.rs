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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawAnalysisTask {
    pub title: String,
    pub project_suggestion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawAnalysis {
    pub summary: String,
    pub mood: String,
    pub emotions: Option<Vec<String>>,
    pub tasks: Option<Vec<RawAnalysisTask>>,
    pub insights: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisResultTask {
    pub title: String,
    pub project_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisResult {
    pub id: String,
    pub summary: String,
    pub mood: String,
    pub emotions: Vec<String>,
    pub tasks: Vec<AnalysisResultTask>,
    pub insights: Vec<String>,
}

impl AnalysisResult {
    pub fn from_raw(raw: RawAnalysis, id: String) -> Result<Self, String> {
        let summary = clean(raw.summary);
        if summary.is_empty() {
            return Err("Analysis response is missing a summary".into());
        }

        let mood = clean(raw.mood);
        if mood.is_empty() {
            return Err("Analysis response is missing a mood".into());
        }

        let tasks = raw.tasks.unwrap_or_default().into_iter().map(|t| {
            AnalysisResultTask {
                title: clean(t.title),
                project_id: t.project_suggestion.unwrap_or_else(|| "inbox".into()),
            }
        }).filter(|t| !t.title.is_empty()).collect();

        Ok(Self {
            id,
            summary,
            mood,
            emotions: raw.emotions.unwrap_or_default().into_iter().map(clean).filter(|s| !s.is_empty()).take(5).collect(),
            tasks,
            insights: raw.insights.unwrap_or_default().into_iter().map(clean).filter(|s| !s.is_empty()).take(3).collect(),
        })
    }
}

fn clean(s: String) -> String {
    s.trim().replace('\n', " ").chars().take(200).collect()
}
