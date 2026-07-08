//! Real-provider compaction smoke test.
//!
//! Run manually with:
//!   cargo test -p agent-diva-agent test_real_compaction -- --nocapture
//!
//! Required:
//!   TEAKACLOUD_API_KEY
//! Optional:
//!   COMPACTION_TEST_MODEL

use agent_diva_agent::compaction::ContextCompactor;
use agent_diva_agent::context_budget::BudgetConfig;
use agent_diva_agent::token_estimate::{estimate_tokens, estimate_total_tokens};
use agent_diva_core::session::{CompactTrigger, Session};
use agent_diva_providers::OpenAiCompatibleClient;
use std::sync::Arc;

fn build_realistic_conversation() -> Vec<(String, String)> {
    vec![
        (
            "user".to_string(),
            "I want to design a Rust desktop AI agent. It should support Windows, macOS, and Linux.".to_string(),
        ),
        (
            "assistant".to_string(),
            "A pragmatic stack is Rust for the backend, Tauri for the desktop shell, Vue and TypeScript for the UI, OpenAI-compatible provider integration, and SQLite for session persistence.".to_string(),
        ),
        (
            "user".to_string(),
            "What are the main risks around IPC, packaging, and plugins?".to_string(),
        ),
        (
            "assistant".to_string(),
            "IPC should stay request-response oriented for predictable latency. Packaging needs a GitHub Actions matrix for the three desktop OS targets. Plugins should start with a narrow trait and a permission model before broad filesystem or process access is allowed.".to_string(),
        ),
        (
            "user".to_string(),
            "I prefer dynamic-library plugins during development, then maybe WASM later.".to_string(),
        ),
        (
            "assistant".to_string(),
            "That is a reasonable staged plan. Use dynamic libraries for native debugging early, keep the public Plugin trait stable, and later add a WASM adapter behind the same trait for safer distribution.".to_string(),
        ),
        (
            "user".to_string(),
            "Summarize the architecture and the remaining tasks.".to_string(),
        ),
        (
            "assistant".to_string(),
            "Architecture: Rust backend, Tauri shell, Vue UI, provider abstraction, SQLite sessions, plugin boundary, and capability-based permissions. Remaining tasks: plugin loader, permission checks, CI packaging, documentation, and performance benchmarks.".to_string(),
        ),
    ]
}

fn conversation_to_session(conversation: &[(String, String)]) -> Session {
    let mut session = Session::new("real-compaction-test");
    for (role, content) in conversation {
        session.add_message(role, content.clone());
    }
    session
}

fn create_real_provider() -> Arc<OpenAiCompatibleClient> {
    let api_key = std::env::var("TEAKACLOUD_API_KEY")
        .expect("TEAKACLOUD_API_KEY is required for this ignored real-provider test");

    let provider = OpenAiCompatibleClient::new(
        Some(api_key),
        Some("http://api.tokenplan.fun:3000/v1".to_string()),
        std::env::var("COMPACTION_TEST_MODEL").unwrap_or_else(|_| "MiniMax-M3".to_string()),
        None,
        Some("teakacloud".to_string()),
        None,
    );

    Arc::new(provider)
}

fn print_separator(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("{}", title);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
#[ignore = "requires TEAKACLOUD_API_KEY"]
async fn test_real_compaction() {
    let conversation = build_realistic_conversation();
    let session = conversation_to_session(&conversation);
    let config = BudgetConfig {
        max_tokens: 100_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.80,
        keep_recent_count: 4,
    };

    let pre_history = session.get_history(50);
    let pre_message_count = pre_history.len();
    let pre_tokens = estimate_total_tokens(&pre_history);
    let pre_chars: usize = pre_history.iter().map(|m| m.content.len()).sum();

    print_separator("before compaction");
    println!("messages: {}", pre_message_count);
    println!("chars: {}", pre_chars);
    println!("estimated tokens: {}", pre_tokens);

    let provider = create_real_provider();
    let compactor = ContextCompactor::new(provider, config);

    let trigger = CompactTrigger::ProactiveThreshold {
        used_tokens: pre_tokens,
        threshold_tokens: (pre_tokens as f64 * 0.8) as usize,
    };

    let result = compactor
        .compact_session(&session, trigger)
        .await
        .expect("compaction should succeed");

    print_separator("compaction result");
    println!("summary chars: {}", result.summary.content.len());
    println!(
        "summary estimated tokens: {}",
        estimate_tokens(&result.summary.content)
    );
    println!("kept recent messages: {}", result.recent_messages.len());

    assert!(!result.summary.content.trim().is_empty());
    assert!(!result.recent_messages.is_empty());
}
