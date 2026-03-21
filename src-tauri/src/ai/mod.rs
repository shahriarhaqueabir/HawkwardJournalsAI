pub mod analysis;
pub mod client;
pub mod companion;
pub mod fallback;
pub mod keywords;
pub mod memory;
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
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub emotions: Option<Vec<String>>,
    #[serde(default)]
    pub tasks: Option<Vec<RawAnalysisTask>>,
    #[serde(default)]
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
    pub fn from_raw(raw: RawAnalysis, id: String, source_text: &str) -> Result<Self, String> {
        let summary = fallback_summary(&raw, source_text);
        if summary.is_empty() {
            return Err("Analysis response is missing a summary".into());
        }

        let mood = fallback_mood(&raw);
        if mood.is_empty() {
            return Err("Analysis response is missing a mood".into());
        }

        let tasks = raw
            .tasks
            .unwrap_or_default()
            .into_iter()
            .map(|t| AnalysisResultTask {
                title: clean(t.title),
                project_id: t.project_suggestion.unwrap_or_else(|| "inbox".into()),
            })
            .filter(|t| !t.title.is_empty())
            .collect();

        Ok(Self {
            id,
            summary,
            mood,
            emotions: raw
                .emotions
                .unwrap_or_default()
                .into_iter()
                .map(clean)
                .filter(|s| !s.is_empty())
                .take(5)
                .collect(),
            tasks,
            insights: raw
                .insights
                .unwrap_or_default()
                .into_iter()
                .map(clean)
                .filter(|s| !s.is_empty())
                .take(3)
                .collect(),
        })
    }
}

fn clean(s: String) -> String {
    s.trim().replace('\n', " ").chars().take(200).collect()
}

fn fallback_summary(raw: &RawAnalysis, source_text: &str) -> String {
    let summary = clean(raw.summary.clone());
    if !summary.is_empty() {
        return summary;
    }

    if let Some(insight) = raw.insights.as_ref().and_then(|items| {
        items.iter().find_map(|item| {
            let cleaned = clean(item.clone());
            (!cleaned.is_empty()).then_some(cleaned)
        })
    }) {
        return insight;
    }

    summarize_source_text(source_text)
}

fn fallback_mood(raw: &RawAnalysis) -> String {
    let mood = clean(raw.mood.clone());
    if !mood.is_empty() {
        return mood;
    }

    raw.emotions
        .as_ref()
        .and_then(|items| {
            items.iter().find_map(|item| {
                let cleaned = clean(item.clone());
                (!cleaned.is_empty()).then_some(cleaned)
            })
        })
        .unwrap_or_else(|| "reflective".to_string())
}

fn summarize_source_text(source_text: &str) -> String {
    let normalized = source_text.split_whitespace().collect::<Vec<_>>().join(" ");

    if normalized.is_empty() {
        return String::new();
    }

    let sentence = normalized
        .split_terminator(['.', '!', '?'])
        .map(str::trim)
        .find(|segment| !segment.is_empty())
        .unwrap_or(normalized.as_str());

    let trimmed = sentence.chars().take(117).collect::<String>();
    let candidate = if sentence.chars().count() > 117 {
        format!("{}...", trimmed.trim_end())
    } else {
        trimmed
    };

    clean(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_missing_summary_from_insight() {
        let raw = RawAnalysis {
            summary: String::new(),
            mood: "anxious".into(),
            emotions: Some(vec!["stress".into()]),
            tasks: None,
            insights: Some(vec![
                "User feels stretched thin and needs firmer boundaries.".into(),
            ]),
        };

        let result = AnalysisResult::from_raw(raw, "entry-1".into(), "Source text").unwrap();
        assert_eq!(
            result.summary,
            "User feels stretched thin and needs firmer boundaries."
        );
    }

    #[test]
    fn fills_missing_summary_from_source_text() {
        let raw = RawAnalysis {
            summary: String::new(),
            mood: "reflective".into(),
            emotions: None,
            tasks: None,
            insights: None,
        };

        let result = AnalysisResult::from_raw(
            raw,
            "entry-1".into(),
            "I had a difficult day at work and realized I need a better shutdown routine.",
        )
        .unwrap();

        assert_eq!(
            result.summary,
            "I had a difficult day at work and realized I need a better shutdown routine"
        );
    }

    #[test]
    fn fills_missing_mood_from_emotions_or_default() {
        let raw = RawAnalysis {
            summary: "Clear summary".into(),
            mood: String::new(),
            emotions: Some(vec![" tired ".into()]),
            tasks: None,
            insights: None,
        };

        let result = AnalysisResult::from_raw(raw, "entry-1".into(), "Source").unwrap();
        assert_eq!(result.mood, "tired");
    }
}
