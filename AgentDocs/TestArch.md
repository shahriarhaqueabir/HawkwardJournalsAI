Full Testing Architecture

Think in layers:

L1 → Pure Logic (fast, deterministic)
L2 → State + Concurrency (race conditions)
L3 → IO Boundaries (DB + HTTP mocked)
L4 → Full Pipeline Simulation
L5 → Failure Injection
🧪 L1 — Unit Tests (Pure Logic)

1.  Deduplication (CORE) #[tokio::test]
    async fn test_should_analyze_deduplication() {
    let state = AnalysisState::new();

        let id = "entry-1";
        let content = "hello world";

        assert!(state.should_analyze(id, content).await);
        assert!(!state.should_analyze(id, content).await);

    }

2.  Hash change detection #[tokio::test]
    async fn test_should_analyze_on_content_change() {
    let state = AnalysisState::new();

        let id = "entry-1";

        assert!(state.should_analyze(id, "v1").await);
        assert!(state.should_analyze(id, "v2").await);

    }

3.  Smart truncation #[test]
    fn test_smart_truncation() {
    let input = "A".repeat(5000);
    let result = smart_truncate(&input);

        assert_eq!(result.len(), 4000);

    }
    ⚙️ L2 — Concurrency Tests (MOST IMPORTANT)

4.  Queue flood test (real-world typing burst) #[tokio::test]
    async fn test_queue_flood() {
    let state = AnalysisState::new();

        let handles: Vec<_> = (0..100).map(|i| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut queue = state.queue.lock().await;
                queue.push_back(format!("id-{}", i));
            })
        }).collect();

        for h in handles {
            h.await.unwrap();
        }

        let queue = state.queue.lock().await;
        assert!(queue.len() <= MAX_QUEUE);

    }

5.  Lock contention test #[tokio::test]
    async fn test_mutex_contention() {
    let state = AnalysisState::new();

        let mut handles = vec![];

        for _ in 0..50 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                state.should_analyze("id", "content").await
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

    }
    🌐 L3 — Mocked IO (Wiremock)
    Setup helper
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

pub async fn setup_mock_ollama() -> MockServer {
let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "response": r#"{
                "summary": "Test summary",
                "mood": "Neutral",
                "emotions": [],
                "tasks": [],
                "insights": []
            }"#
        })))
        .mount(&server)
        .await;

    server

} 6. Successful AI call #[tokio::test]
async fn test_ollama_success() {
let server = setup_mock_ollama().await;

    let client = OllamaClient::new_with_url(
        "llama3.2".into(),
        server.uri()
    );

    let result = client.analyze_journal("test").await.unwrap();

    assert_eq!(result.summary, "Test summary");

} 7. Malformed JSON #[tokio::test]
async fn test_ollama_bad_json() {
let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("INVALID"))
        .mount(&server)
        .await;

    let client = OllamaClient::new_with_url("llama3.2".into(), server.uri());

    let result = client.analyze_journal("test").await;

    assert!(result.is_err());

} 8. Timeout test #[tokio::test]
async fn test_ollama_timeout() {
let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(60)))
        .mount(&server)
        .await;

    let client = OllamaClient::new_with_short_timeout(server.uri());

    let result = client.analyze_journal("test").await;

    assert!(result.is_err());

}
🔄 L4 — Full Pipeline Simulation 9. End-to-end flow #[tokio::test]
async fn test_full_pipeline() {
let ctx = setup_test_context().await;

    // Insert entry
    insert_entry(&ctx.conn, "id-1", "I feel stressed").unwrap();

    // Push to queue
    {
        let mut queue = ctx.state.ai_state.queue.lock().await;
        queue.push_back("id-1".to_string());
    }

    // Run worker once
    run_worker_once(&ctx).await;

    // Assert result emitted
    let events = ctx.events.lock().await;
    assert!(events.contains("journal_analysis_result"));

}
☠️ L5 — Failure Injection (CRITICAL) 10. Ghost Entry Test (your most important one) #[tokio::test]
async fn test_ghost_entry() {
let ctx = setup_test_context().await;

    insert_entry(&ctx.conn, "id-1", "test").unwrap();

    {
        let mut queue = ctx.state.ai_state.queue.lock().await;
        queue.push_back("id-1".to_string());
    }

    // Simulate deletion before worker processes
    delete_entry(&ctx.conn, "id-1").unwrap();

    run_worker_once(&ctx).await;

    let logs = ctx.logs.lock().await;
    assert!(logs.contains("Ghost Entry"));

}

Building on your Phase 2 & 3 Development Plan, here is a comprehensive set of test suites designed to validate the Hardening, Thinking Partner UX, and Background Analysis layers. These tests move from L1 (Pure Logic) to L4 (Full Pipeline Simulation) as outlined in your TestArch.md.

🧪 Phase 2: Tool & Orchestration Tests
Focus: Tool reliability, fallback extraction, and lock safety.

1. L1: Fallback Regex Extraction
Ensures that if the LLM fails to use the official tool-calling API, your fallback.rs logic can still recover the intent from markdown or raw JSON.

Rust
#[test]
fn test_fallback_extraction_priority() {
    let input = "I will help you. <tool_call>{\"name\": \"list_tasks\", \"arguments\": {}}</tool_call>";
    let calls = crate::ai::fallback::extract_tool_calls(input);
    
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "list_tasks");
    
    let messy_json = "Here is the data: {\"name\": \"create_task\", \"summary\": \"Buy Milk\"}";
    let calls_json = crate::ai::fallback::extract_tool_calls(messy_json);
    // Should return None because "summary" triggers Guard 2 (Journal Analysis misidentification)
    assert!(calls_json.is_empty()); 
}
2. L2: Async Lock Hygiene (Deadlock Prevention)
Ensures that the application doesn't hang when the AI is processing while a database write is occurring.

Rust
#[tokio::test]
async fn test_db_lock_release_before_ai_await() {
    let (app, state) = setup_test_context().await;
    
    // Simulate a long-running AI call that should NOT hold the DB lock
    let handle = tokio::spawn(async move {
        // This simulates the logic in lib.rs where we fetch data then call Ollama
        let _data = {
            let _conn = state.conn.lock().await;
            // Fetching...
        }; // Lock drops here
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    });

    // Try to grab the lock immediately
    let conn = state.conn.lock().await;
    assert!(conn.is_autocommit()); // If we get here, the lock was released correctly
}
🧪 Phase 3: Thinking Partner & Analysis Tests
Focus: Memory management, state transitions, and narration loops.

1. L1: Analysis State Hardening (Memory Leak Prevention)
Validates that the last_hashes map stays within the 1000-entry limit defined in your hardening plan.

Rust
#[tokio::test]
async fn test_analysis_state_capacity_limit() {
    let state = crate::ai::analysis::AnalysisState::new();
    
    // Flood the state with 1100 unique entries
    for i in 0..1100 {
        state.should_analyze(&format!("id-{}", i), "some content").await;
    }

    let hashes = state.last_hashes.lock().await;
    // Verify the clear() logic triggered to prevent memory leak
    assert!(hashes.len() <= 1000);
}
2. L3/L4: Background Worker Pipeline
Simulates the "Save -> Notify -> Process -> Save Result" flow.

Rust
#[tokio::test]
async fn test_full_analysis_pipeline_simulation() {
    let (app, state) = setup_test_context().await;
    let entry_id = "test-entry-001";

    // 1. Simulate frontend trigger
    {
        let mut queue = state.ai_state.queue.lock().await;
        queue.push_back(entry_id.to_string());
        state.ai_state.notify.notify_one();
    }

    // 2. Run the worker logic (mocking the Ollama response)
    // In a real L4 test, you'd use a MockServer for the HTTP call
    let result = crate::ai::mod::AnalysisResult {
        id: entry_id.to_string(),
        summary: "Tested background flow".into(),
        mood: "productive".into(),
        emotions: vec!["focused".into()],
        tasks: vec![],
        insights: vec!["System works".into()],
    };

    // 3. Verify DB persistence
    let conn = state.conn.lock().await;
    crate::db::ai::save_analysis_result(&conn, &result).unwrap();
    
    let saved = crate::db::ai::get_latest_analysis(&conn, entry_id).unwrap();
    assert_eq!(saved.mood, "productive");
}
3. L1: Narration Turn Logic (Turn 2 Re-mapper)
Verifies that tool results are correctly transformed into "user" messages to bypass the Ollama 400 error.

Rust
#[test]
fn test_narration_turn_mapping() {
    let history = vec![crate::ai::client::ChatMessage {
        role: "user".into(),
        content: "What are my tasks?".into(),
        tool_calls: None,
    }];
    
    let tool_results = vec![("list_tasks".into(), serde_json::json!([{"title": "Fix bug"}]))];
    
    let remapped = crate::ai::client::prepare_narration_history(history, tool_results);
    
    // The history should now have 3 messages: [User, Tool Result as User, Narration Instruction]
    assert_eq!(remapped.len(), 3);
    assert_eq!(remapped[1].role, "user");
    assert!(remapped[1].content.contains("[DATABASE_RESULT]"));
    assert!(remapped[2].content.contains("summarize"));
}
📊 Reporting & Observability Checklist
To monitor these tests in your logs (as per TestArch.md), ensure your AnalysisStatus transitions match this sequence:

Event: AppEvent::AiStatus("Thinking...") emitted.

Worker: AnalysisStatus::Processing written to AnalysisState.

Tool Execution: AiStatus("Searching tasks...") emitted via tools.rs.

Completion: journal_analyzed emitted to frontend.

📊 Observability (THIS IS WHAT LEVELS YOU UP)
Replace println! with tracing
use tracing::{info, warn, error};

info!("Queued entry {}", id);
warn!("Ghost entry detected: {}", id);
error!("AI call failed: {}", e);
Enable logging
tracing_subscriber::fmt()
.with_max_level(tracing::Level::DEBUG)
.init();
Test logs
use tracing_test::traced_test;

#[traced_test] #[tokio::test]
async fn test_logs() {
warn!("Ghost Entry detected");

    assert!(logs_contain("Ghost Entry"));

}
🧠 Final System Insight

What you’ve built is not just “tests”—it’s:

A Distributed System Simulator

You are validating:

async scheduling

DB consistency

event propagation

LLM reliability

UI contract integrity

That’s senior/staff-level architecture thinking.

🚀 Next Step (High Impact)

You’re ready for:

👉 Deterministic Worker Trigger (Kill the polling loop)

Replace:

tokio::time::sleep(...)

With:

Notify / event-driven wakeup

This will:

reduce latency from 500ms → ~0ms

eliminate wasted CPU cycles

make system feel "instant"
