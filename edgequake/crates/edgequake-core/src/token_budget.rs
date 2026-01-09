//! Token budget management for LLM context windows.
//!
//! This module provides utilities for managing token budgets when constructing
//! prompts and context for LLM queries. It ensures that context fits within
//! model-specific token limits.
//!
//! ## Implements
//!
//! - **FEAT0840**: Token counting with tiktoken
//! - **FEAT0841**: Budget allocation across content sources
//! - **FEAT0842**: Text truncation to fit limits
//!
//! ## Use Cases
//!
//! - **UC2430**: System counts tokens in context
//! - **UC2431**: System allocates budget across entities/chunks
//! - **UC2432**: System truncates content to fit model limit
//!
//! ## Enforces
//!
//! - **BR0840**: Token counts must use model-specific encoder
//! - **BR0841**: Reserved space for response must be maintained
//!
//! Based on LightRAG's context management in `lightrag/operate.py`

use tiktoken_rs::{get_bpe_from_model, CoreBPE};

/// Manages token budgets for context construction.
///
/// Provides token counting, truncation, and budget allocation across
/// multiple content sources (entities, relationships, chunks).
pub struct TokenBudget {
    encoder: CoreBPE,
    max_tokens: usize,
    reserved_for_response: usize,
    reserved_for_prompt: usize,
}

impl TokenBudget {
    /// Create a new token budget for a specific model.
    ///
    /// # Arguments
    /// * `model` - The model name (e.g., "gpt-4", "gpt-4o-mini", "gpt-3.5-turbo")
    /// * `max_tokens` - Maximum context window size
    ///
    /// # Example
    /// ```ignore
    /// let budget = TokenBudget::new("gpt-4o-mini", 128000);
    /// ```
    pub fn new(model: &str, max_tokens: usize) -> Self {
        let encoder = get_bpe_from_model(model)
            .unwrap_or_else(|_| get_bpe_from_model("gpt-4").expect("gpt-4 encoder must exist"));

        Self {
            encoder,
            max_tokens,
            reserved_for_response: 1000,
            reserved_for_prompt: 500,
        }
    }

    /// Create with custom reserved token amounts.
    pub fn with_reserves(mut self, response: usize, prompt: usize) -> Self {
        self.reserved_for_response = response;
        self.reserved_for_prompt = prompt;
        self
    }

    /// Get the maximum token limit.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Get available tokens for context (excluding reserved tokens).
    pub fn available_tokens(&self) -> usize {
        self.max_tokens
            .saturating_sub(self.reserved_for_response)
            .saturating_sub(self.reserved_for_prompt)
    }

    /// Count tokens in a text string.
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_with_special_tokens(text).len()
    }

    /// Check if text fits within a given budget.
    pub fn fits_in_budget(&self, text: &str, budget: usize) -> bool {
        self.count_tokens(text) <= budget
    }

    /// Truncate text to fit within a token budget.
    ///
    /// Attempts to truncate at sentence boundaries when possible.
    pub fn truncate_to_budget(&self, text: &str, budget: usize) -> String {
        let tokens = self.encoder.encode_with_special_tokens(text);

        if tokens.len() <= budget {
            return text.to_string();
        }

        // Truncate tokens
        let truncated_tokens: Vec<_> = tokens.into_iter().take(budget).collect();

        match self.encoder.decode(truncated_tokens) {
            Ok(decoded) => {
                // Try to end at a sentence boundary
                if let Some(last_period) = decoded.rfind(". ") {
                    decoded[..=last_period].to_string()
                } else if let Some(last_newline) = decoded.rfind('\n') {
                    decoded[..last_newline].to_string()
                } else {
                    decoded
                }
            }
            Err(_) => {
                // Fallback: rough character-based truncation
                let char_estimate = budget * 4;
                text.chars().take(char_estimate).collect()
            }
        }
    }

    /// Allocate budget proportionally across multiple content sources.
    ///
    /// Each source has a weight that determines its share of the budget.
    /// Minimum allocations are respected if possible.
    pub fn allocate_budget(&self, sources: &[BudgetSource]) -> Vec<BudgetAllocation> {
        let total_available = self.available_tokens();

        if sources.is_empty() {
            return vec![];
        }

        let total_weight: f64 = sources.iter().map(|s| s.weight).sum();
        let total_min: usize = sources.iter().map(|s| s.min_tokens).sum();

        // If minimums exceed budget, allocate proportionally to minimums
        if total_min >= total_available {
            return sources
                .iter()
                .map(|s| BudgetAllocation {
                    name: s.name.clone(),
                    tokens: (s.min_tokens as f64 / total_min as f64 * total_available as f64)
                        as usize,
                })
                .collect();
        }

        // Allocate remaining budget after minimums
        let remaining = total_available - total_min;

        sources
            .iter()
            .map(|s| {
                let weighted_share = (s.weight / total_weight * remaining as f64) as usize;
                BudgetAllocation {
                    name: s.name.clone(),
                    tokens: s.min_tokens + weighted_share,
                }
            })
            .collect()
    }

    /// Build context within budget from multiple sources.
    ///
    /// Returns the concatenated context string and actual tokens used.
    pub fn build_context(&self, sources: &[ContextSource]) -> (String, usize) {
        let allocations = self.allocate_budget(
            &sources
                .iter()
                .map(|s| BudgetSource {
                    name: s.name.clone(),
                    weight: s.weight,
                    min_tokens: s.min_tokens,
                })
                .collect::<Vec<_>>(),
        );

        let mut context = String::new();
        let mut total_tokens = 0;

        for (source, allocation) in sources.iter().zip(allocations.iter()) {
            if source.content.is_empty() {
                continue;
            }

            let truncated = self.truncate_to_budget(&source.content, allocation.tokens);
            let tokens = self.count_tokens(&truncated);

            if !truncated.is_empty() {
                if !context.is_empty() {
                    context.push_str("\n\n");
                    total_tokens += 2; // Approximate for newlines
                }
                context.push_str(&truncated);
                total_tokens += tokens;
            }
        }

        (context, total_tokens)
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::new("gpt-4o-mini", 128_000)
    }
}

/// Source for budget allocation.
#[derive(Debug, Clone)]
pub struct BudgetSource {
    /// Name of the source (for debugging/logging).
    pub name: String,
    /// Weight for proportional allocation.
    pub weight: f64,
    /// Minimum tokens to allocate.
    pub min_tokens: usize,
}

impl Default for BudgetSource {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight: 1.0,
            min_tokens: 100,
        }
    }
}

/// Result of budget allocation.
#[derive(Debug, Clone)]
pub struct BudgetAllocation {
    /// Name of the allocated source.
    pub name: String,
    /// Tokens allocated to this source.
    pub tokens: usize,
}

/// Content source with budget metadata.
#[derive(Debug, Clone)]
pub struct ContextSource {
    /// Name of the source.
    pub name: String,
    /// Content to include.
    pub content: String,
    /// Weight for proportional allocation.
    pub weight: f64,
    /// Minimum tokens to allocate.
    pub min_tokens: usize,
}

impl ContextSource {
    /// Create a new context source.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            weight: 1.0,
            min_tokens: 100,
        }
    }

    /// Set the weight for this source.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set the minimum tokens for this source.
    pub fn with_min_tokens(mut self, min: usize) -> Self {
        self.min_tokens = min;
        self
    }
}

/// Predefined token limits for common models.
pub mod model_limits {
    /// GPT-4 Turbo context limit
    pub const GPT4_TURBO: usize = 128_000;
    /// GPT-4o context limit
    pub const GPT4O: usize = 128_000;
    /// GPT-4o Mini context limit
    pub const GPT4O_MINI: usize = 128_000;
    /// GPT-3.5 Turbo context limit
    pub const GPT35_TURBO: usize = 16_385;
    /// Claude 3 Opus context limit
    pub const CLAUDE3_OPUS: usize = 200_000;
    /// Claude 3 Sonnet context limit
    pub const CLAUDE3_SONNET: usize = 200_000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let text = "Hello, world!";
        let count = budget.count_tokens(text);
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_available_tokens() {
        let budget = TokenBudget::new("gpt-4", 4000).with_reserves(1000, 500);
        assert_eq!(budget.available_tokens(), 2500);
    }

    #[test]
    fn test_fits_in_budget() {
        let budget = TokenBudget::new("gpt-4", 4000);
        assert!(budget.fits_in_budget("Hello", 100));
        assert!(!budget.fits_in_budget("Hello", 0));
    }

    #[test]
    fn test_truncate_to_budget() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let long_text = "This is a sentence. This is another sentence. And a third one.";
        let truncated = budget.truncate_to_budget(long_text, 5);
        assert!(budget.count_tokens(&truncated) <= 5);
    }

    #[test]
    fn test_budget_allocation() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let sources = vec![
            BudgetSource {
                name: "entities".to_string(),
                weight: 2.0,
                min_tokens: 100,
            },
            BudgetSource {
                name: "chunks".to_string(),
                weight: 1.0,
                min_tokens: 100,
            },
        ];

        let allocations = budget.allocate_budget(&sources);
        assert_eq!(allocations.len(), 2);
        assert!(allocations[0].tokens > allocations[1].tokens); // Entities get 2x weight
    }

    #[test]
    fn test_budget_allocation_respects_minimums() {
        let budget = TokenBudget::new("gpt-4", 1000).with_reserves(0, 0);
        let sources = vec![
            BudgetSource {
                name: "a".to_string(),
                weight: 1.0,
                min_tokens: 200,
            },
            BudgetSource {
                name: "b".to_string(),
                weight: 1.0,
                min_tokens: 200,
            },
        ];

        let allocations = budget.allocate_budget(&sources);
        assert!(allocations[0].tokens >= 200);
        assert!(allocations[1].tokens >= 200);
    }

    #[test]
    fn test_build_context() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let sources = vec![
            ContextSource::new("intro", "Introduction text here.").with_weight(1.0),
            ContextSource::new("body", "Body content goes here.").with_weight(2.0),
        ];

        let (context, tokens) = budget.build_context(&sources);
        assert!(!context.is_empty());
        assert!(tokens > 0);
        assert!(context.contains("Introduction"));
        assert!(context.contains("Body"));
    }

    #[test]
    fn test_empty_sources() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let allocations = budget.allocate_budget(&[]);
        assert!(allocations.is_empty());
    }

    #[test]
    fn test_default_budget() {
        let budget = TokenBudget::default();
        assert_eq!(budget.max_tokens(), 128_000);
    }
}
