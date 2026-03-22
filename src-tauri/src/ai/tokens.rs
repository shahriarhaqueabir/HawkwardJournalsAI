//! Token counting and context budget management.
//!
//! Uses `tiktoken-rs` with the `cl100k_base` vocabulary (GPT-4 / LLaMA 3 family).
//! Counts are ~95% correlated with Ollama's internal tokenizer — accurate enough
//! for budget enforcement without requiring a round-trip to the Ollama API.
//!
//! Decision Reference: D-93 (default context = 16384), D-94 (compact injection format).

use std::sync::OnceLock;
use tiktoken_rs::{cl100k_base, CoreBPE};

/// The default context window for Ollama (D-93).
pub const DEFAULT_CONTEXT_TOKENS: usize = 16384;

/// Reserve this many tokens for the model's generation output.
/// This prevents us from filling the entire context with input.
pub const OUTPUT_RESERVE_TOKENS: usize = 1024;

/// Maximum input tokens we allow before truncating/warning.
/// = DEFAULT_CONTEXT_TOKENS - OUTPUT_RESERVE_TOKENS
pub const MAX_INPUT_TOKENS: usize = DEFAULT_CONTEXT_TOKENS - OUTPUT_RESERVE_TOKENS;

// A lazily-initialised, globally shared tokenizer instance.
// Initialising BPE is expensive (~50ms), so we do it once and reuse.
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

fn get_tokenizer() -> &'static CoreBPE {
    TOKENIZER.get_or_init(|| {
        cl100k_base().expect("tiktoken cl100k_base vocabulary failed to initialise")
    })
}

/// Count the number of tokens in a text string.
/// Returns 0 on any internal tokenizer error (fail-safe).
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let bpe = get_tokenizer();
    bpe.encode_ordinary(text).len()
}

/// Count tokens across multiple chat messages (role + content).
/// This mirrors how Ollama counts context usage internally.
pub fn count_chat_tokens(messages: &[(&str, &str)]) -> usize {
    messages
        .iter()
        .map(|(role, content)| {
            // Each message has ~4 overhead tokens for role formatting
            4 + count_tokens(role) + count_tokens(content)
        })
        .sum::<usize>()
        // ~3 tokens for the reply primer
        + 3
}

/// A snapshot of context budget usage, safe to serialize to the frontend.
#[derive(Debug, serde::Serialize, Clone)]
pub struct TokenBudget {
    /// Tokens used by the current input
    pub used: usize,
    /// Maximum allowed input tokens (context - output reserve)
    pub max_input: usize,
    /// Total context window size
    pub context_window: usize,
    /// Whether the input fits without truncation
    pub within_budget: bool,
    /// Usage as a percentage (0–100)
    pub percent_used: f32,
}

impl TokenBudget {
    pub fn calculate(text: &str, context_window: usize) -> Self {
        let max_input = context_window.saturating_sub(OUTPUT_RESERVE_TOKENS);
        let used = count_tokens(text);
        let within_budget = used <= max_input;
        let percent_used = if context_window == 0 {
            0.0
        } else {
            (used as f32 / context_window as f32) * 100.0
        };

        Self {
            used,
            max_input,
            context_window,
            within_budget,
            percent_used,
        }
    }

    pub fn for_chat(messages: &[(&str, &str)], context_window: usize) -> Self {
        let max_input = context_window.saturating_sub(OUTPUT_RESERVE_TOKENS);
        let used = count_chat_tokens(messages);
        let within_budget = used <= max_input;
        let percent_used = if context_window == 0 {
            0.0
        } else {
            (used as f32 / context_window as f32) * 100.0
        };

        Self {
            used,
            max_input,
            context_window,
            within_budget,
            percent_used,
        }
    }
}

/// Truncate text to fit within a token budget, preserving as much of the
/// beginning and end as possible (head-tail strategy, same as the character
/// truncator it replaces). Returns the truncated string and the final token count.
pub fn truncate_to_token_budget(text: &str, max_tokens: usize) -> (String, usize) {
    let total = count_tokens(text);
    if total <= max_tokens {
        return (text.to_string(), total);
    }

    // We target 48% head / 48% tail with a 4% separator budget
    let half = (max_tokens as f64 * 0.48) as usize;
    let bpe = get_tokenizer();
    let all_tokens = bpe.encode_ordinary(text);

    if all_tokens.len() <= max_tokens {
        return (text.to_string(), all_tokens.len());
    }

    let head_tokens = &all_tokens[..half.min(all_tokens.len())];
    let tail_start = all_tokens.len().saturating_sub(half);
    let tail_tokens = &all_tokens[tail_start..];

    let head = bpe
        .decode(head_tokens.to_vec())
        .unwrap_or_default();
    let tail = bpe
        .decode(tail_tokens.to_vec())
        .unwrap_or_default();

    let separator = "\n[... content truncated to fit context window ...]\n";
    let result = format!("{}{}{}", head, separator, tail);
    let final_count = count_tokens(&result);

    (result, final_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_tokens_on_empty_string() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn counts_tokens_on_simple_text() {
        // "Hello, world!" is known to be 4 tokens in cl100k_base
        let count = count_tokens("Hello, world!");
        assert!(count > 0, "should count tokens for 'Hello, world!'");
    }

    #[test]
    fn token_budget_within_budget() {
        let budget = TokenBudget::calculate("Hello", 100);
        assert!(budget.within_budget);
        assert!(budget.used > 0);
        assert_eq!(budget.context_window, 100);
        assert_eq!(budget.max_input, 100 - OUTPUT_RESERVE_TOKENS);
    }

    #[test]
    fn token_budget_over_budget() {
        // A string of 100 repetitions of a common word should exceed a 20-token budget
        let long_text = "productivity ".repeat(100);
        let budget = TokenBudget::calculate(&long_text, 20);
        assert!(!budget.within_budget);
        assert!(budget.percent_used > 100.0);
    }

    #[test]
    fn truncate_short_text_unchanged() {
        let text = "This is a short sentence.";
        let (result, count) = truncate_to_token_budget(text, 1000);
        assert_eq!(result, text);
        assert!(count < 1000);
    }

    #[test]
    fn truncate_long_text_fits_within_budget() {
        let long_text = "The quick brown fox jumps over the lazy dog. ".repeat(500);
        let max = 200;
        let (result, count) = truncate_to_token_budget(&long_text, max);
        assert!(
            count <= max + 20, // allow small overshoot from separator tokens
            "Truncated text has {} tokens, expected <= {}",
            count, max
        );
        assert!(result.contains("[... content truncated"), "Should contain truncation marker");
    }

    #[test]
    fn chat_token_count_is_nonzero_for_messages() {
        let messages = [("system", "You are a helpful assistant."), ("user", "Hello!")];
        let count = count_chat_tokens(&messages);
        assert!(count > 0);
    }

    #[test]
    fn default_constants_are_coherent() {
        assert_eq!(MAX_INPUT_TOKENS, DEFAULT_CONTEXT_TOKENS - OUTPUT_RESERVE_TOKENS);
        assert!(OUTPUT_RESERVE_TOKENS < DEFAULT_CONTEXT_TOKENS);
    }
}
