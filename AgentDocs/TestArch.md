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
