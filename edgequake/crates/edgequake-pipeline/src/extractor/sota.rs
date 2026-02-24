//! SOTA LLM-based entity extractor using tuple-format prompts.
//!
//! @implements FEAT0303

use async_trait::async_trait;
use edgequake_llm::traits::{ChatMessage, CompletionOptions};

use super::{EntityExtractor, ExtractionResult};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

/// SOTA LLM-based entity extractor using tuple-format prompts.
///
/// This extractor uses the SOTA prompt system ported from LightRAG,
/// featuring tuple-based output format for more robust parsing.
pub struct SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
    prompts: crate::prompts::EntityExtractionPrompts,
    parser: crate::prompts::HybridExtractionParser,
    language: String,
}

impl<L> SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create a new SOTA extractor with default settings.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: crate::prompts::default_entity_types(),
            prompts: crate::prompts::EntityExtractionPrompts::default(),
            parser: crate::prompts::HybridExtractionParser::new(true),
            language: "English".to_string(),
        }
    }

    /// Set custom entity types.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set output language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set custom prompts.
    pub fn with_prompts(mut self, prompts: crate::prompts::EntityExtractionPrompts) -> Self {
        self.prompts = prompts;
        self
    }
}

#[async_trait]
impl<L> EntityExtractor for SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();

        // Pre-validate chunk size to fail fast on oversized chunks
        // WHY: Large chunks (>1500 tokens ~6KB) likely exceed LLM timeout (120s)
        // Based on LightRAG research: 1200-1500 tokens is optimal for reliability
        // This prevents wasting API calls and provides immediate, actionable feedback
        let chunk_size_bytes = chunk.content.len();
        let estimated_tokens = chunk_size_bytes / 4; // Rough estimate: 1 token ≈ 4 chars
        const MAX_CHUNK_TOKENS: usize = 1500; // Reduced from 4000 based on research

        if estimated_tokens > MAX_CHUNK_TOKENS {
            // Calculate adaptive chunk size based on estimated document size
            let recommended_chunk_size = if chunk_size_bytes > 100_000 {
                600 // >100KB: minimal chunks
            } else if chunk_size_bytes > 50_000 {
                800 // 50-100KB: reduced chunks
            } else {
                1200 // <50KB: standard chunks
            };

            let error_msg = format!(
                "Chunk too large for LLM processing. Chunk size: {}KB (~{} tokens, max: {}). \
                Suggestions:\n\
                1. Use adaptive chunking: Set chunk_size={} for this document size\n\
                2. Split document into smaller files (<50KB each)\n\
                3. Current config uses chunk_size=1200 which is too large for {}KB documents\n\
                4. Alternative: Use Ollama with 300s timeout instead of OpenAI (120s)\n\
                Chunk ID: {} | Document size: ~{}KB",
                chunk_size_bytes / 1024,
                estimated_tokens,
                MAX_CHUNK_TOKENS,
                recommended_chunk_size,
                chunk_size_bytes / 1024,
                chunk.id,
                chunk_size_bytes / 1024
            );
            tracing::error!(
                chunk_id = %chunk.id,
                chunk_size_bytes = chunk_size_bytes,
                estimated_tokens = estimated_tokens,
                max_tokens = MAX_CHUNK_TOKENS,
                "{}",
                error_msg
            );
            return Err(PipelineError::Validation(error_msg));
        }

        // Build system and user prompts
        let system_prompt = self
            .prompts
            .system_prompt(&self.entity_types, &self.language);
        let user_prompt =
            self.prompts
                .user_prompt(&chunk.content, &self.entity_types, &self.language);

        // Create chat messages for system + user prompt
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        // ═══════════════════════════════════════════════════════════════════════════════
        // WHY DO WE NEED ADAPTIVE MAX_TOKENS? (INPUT vs OUTPUT Token Management)
        // ═══════════════════════════════════════════════════════════════════════════════
        //
        // PROBLEM: Document chunking (INPUT) ≠ Response size management (OUTPUT)
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ INPUT SIDE (Document Chunking) - Already Working ✅                      │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   137KB Document (1234 lines)
        //         │
        //         ├─> Chunk 1: ~5KB (~1200 tokens) ──┐
        //         ├─> Chunk 2: ~5KB (~1200 tokens) ──┤
        //         ├─> Chunk 3: ~5KB (~1200 tokens) ──┤─> Process each chunk
        //         ├─> Chunk 4: ~5KB (~1200 tokens) ──┤   separately with LLM
        //         └─> Chunk 5: ~5KB (~1200 tokens) ──┘
        //
        //   INPUT chunking prevents LLM from being overwhelmed by large documents.
        //   Orchestrator uses adaptive INPUT chunk sizes: 600-1200 tokens based on doc size.
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ OUTPUT SIDE (LLM Response) - The ACTUAL Problem We Fix Here! ❌ → ✅     │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   Single 5KB Chunk (~1500 tokens INPUT)
        //         │
        //         ├─> LLM Entity Extraction
        //         │
        //         └─> JSON Response (OUTPUT): {
        //               "entities": [
        //                 {"name": "SARAH_CHEN", "type": "PERSON", ...},
        //                 {"name": "NEURAL_NETWORK", "type": "CONCEPT", ...},
        //                 {"name": "GRADIENT_DESCENT", "type": "METHOD", ...},
        //                 ... 50+ more entities ...
        //               ],
        //               "relationships": [
        //                 {"source": "SARAH_CHEN", "target": "NEURAL_NETWORK", ...},
        //                 ... 100+ more relationships ...
        //               ]
        //             }
        //
        //   OUTPUT Response Size: ~9,000 tokens (6x larger than INPUT!)
        //                         ▲
        //                         │
        //                 EXCEEDED 8192 TOKEN LIMIT!
        //                         │
        //   Result: JSON truncated mid-array → "EOF while parsing a list at line 984"
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ KEY INSIGHT: Small INPUT ≠ Small OUTPUT                                  │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   • Academic papers/technical docs have HIGH entity density
        //   • Single 1500-token chunk can generate 50+ entities + 100+ relationships
        //   • Complex JSON structure multiplies output size (IDs, types, descriptions)
        //   • SafetyLimitedProvider DEFAULT_MAX_TOKENS=8192 was TOO LOW for OUTPUT
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ SOLUTION: Adaptive max_tokens for OUTPUT Responses                      │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   We calculate base_max_tokens based on CHUNK COMPLEXITY (not just size):
        //
        //     <25KB chunks  → 4096 tokens  (simple content, few entities)
        //     25-75KB       → 8192 tokens  (medium complexity)
        //     75-125KB      → 12288 tokens (high entity density)
        //     >125KB        → 16384 tokens (very complex, many entities)
        //
        //   PLUS progressive retry logic:
        //   • Detect truncation via finish_reason="length" or JSON parse errors
        //   • Double max_tokens on retry (up to 32768 maximum)
        //   • Retry up to 3 times with exponential backoff
        //
        //   Result: Adaptive OUTPUT limits match ACTUAL response complexity!
        //
        // ═══════════════════════════════════════════════════════════════════════════

        let base_max_tokens = if chunk_size_bytes < 25_000 {
            4096 // <25KB: small documents, likely simple content
        } else if chunk_size_bytes < 75_000 {
            8192 // 25-75KB: medium documents, moderate entity density
        } else if chunk_size_bytes < 125_000 {
            12288 // 75-125KB: large documents, high entity density
        } else {
            16384 // >125KB: very large documents, very high entity density
        };

        // ═══════════════════════════════════════════════════════════════════════════════
        // RETRY STRATEGY: Progressive max_tokens Increase on Truncation Detection
        // ═══════════════════════════════════════════════════════════════════════════════
        //
        // ┌─────────────────────────────────────────────────────────────────────────┐
        // │ Adaptive Retry Flow (Exponential Backoff + Progressive Token Increase)   │
        // └─────────────────────────────────────────────────────────────────────────┘
        //
        //   Attempt 1: base_max_tokens (e.g., 8192)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Check finish_reason
        //      │   │
        //      │   ├─> "stop" → Success ✅ Parse JSON
        //      │   │
        //      │   └─> "length" → Truncated! ❌
        //      │       │
        //      │       └─> Sleep 100ms, Retry with 16384 tokens
        //      │
        //   Attempt 2: 2x tokens (16384)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Parse JSON
        //      │   │
        //      │   ├─> Success ✅ Return result
        //      │   │
        //      │   └─> "EOF while parsing" ❌
        //      │       │
        //      │       └─> Detected JSON truncation → Sleep 200ms, Retry with 32768
        //      │
        //   Attempt 3: 4x tokens (32768 MAX)
        //      │
        //      ├─> LLM Call
        //      │
        //      ├─> Parse JSON
        //      │   │
        //      │   ├─> Success ✅ Return result
        //      │   │
        //      │   └─> Still truncated ❌
        //      │       │
        //      │       └─> ERROR: Document too complex, suggest splitting
        //
        // WHY PROGRESSIVE INCREASE?
        // • Start conservative to save API costs (smaller tokens = cheaper)
        // • Only increase when PROVEN necessary (truncation detected)
        // • Cap at 32768 to prevent runaway costs
        // • Exponential backoff (100ms → 200ms → 400ms) prevents API rate limits
        //
        // TRUNCATION DETECTION:
        // 1. finish_reason="length" → LLM hit max_tokens limit (lines 807-830)
        // 2. JSON parse errors (EOF, unclosed) → Response truncated mid-JSON (lines 897-928)
        //
        // ═══════════════════════════════════════════════════════════════════════════

        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;
        let mut current_max_tokens = base_max_tokens;

        for attempt in 1..=MAX_RETRIES {
            // Create completion options with adaptive max_tokens
            let options = CompletionOptions {
                max_tokens: Some(current_max_tokens),
                temperature: Some(0.0), // Deterministic for extraction
                ..Default::default()
            };

            tracing::debug!(
                attempt = attempt,
                chunk_id = %chunk.id,
                chunk_size_kb = chunk_size_bytes / 1024,
                max_tokens = current_max_tokens,
                "Making LLM call with adaptive max_tokens"
            );

            // Make LLM call using chat interface with options
            let response = match self.llm_provider.chat(&messages, Some(&options)).await {
                Ok(resp) => resp,
                Err(e) => {
                    let error_str = e.to_string().to_lowercase();
                    let is_timeout =
                        error_str.contains("timeout") || error_str.contains("timed out");

                    tracing::warn!(
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        chunk_id = %chunk.id,
                        chunk_size_kb = chunk_size_bytes / 1024,
                        estimated_tokens = estimated_tokens,
                        is_timeout = is_timeout,
                        "LLM call failed, retrying..."
                    );

                    // Build enhanced error message with diagnostic info
                    let enhanced_error = if is_timeout {
                        // Calculate adaptive chunk size recommendation
                        let recommended_chunk_size = if chunk_size_bytes > 100_000 {
                            600 // >100KB: minimal chunks
                        } else if chunk_size_bytes > 50_000 {
                            800 // 50-100KB: reduced chunks
                        } else {
                            1200 // <50KB: standard chunks
                        };

                        format!(
                            "LLM timeout after 120s. Chunk: {}KB (~{} tokens, max: {}). \
                            Document appears too large for current settings. \
                            Suggestions:\n\
                            1. BEST: Use adaptive chunking with chunk_size={} (recommended for {}KB documents)\n\
                            2. Split document into smaller files (<50KB each)\n\
                            3. Switch to Ollama provider (300s timeout vs OpenAI 120s)\n\
                            4. Alternative: Increase LLM_TIMEOUT_SECS env variable (not recommended)\n\
                            Chunk ID: {} | Attempt: {}/{} | Document size: ~{}KB | Original error: {}",
                            chunk_size_bytes / 1024,
                            estimated_tokens,
                            MAX_CHUNK_TOKENS,
                            recommended_chunk_size,
                            chunk_size_bytes / 1024,
                            chunk.id,
                            attempt,
                            MAX_RETRIES,
                            chunk_size_bytes / 1024,
                            e
                        )
                    } else {
                        format!("LLM error: {}", e)
                    };

                    last_error = Some(PipelineError::ExtractionError(enhanced_error));
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    } else {
                        return Err(last_error.unwrap());
                    }
                }
            };

            // DEBUG: Log raw LLM response for debugging JSON parsing errors
            tracing::debug!(
                chunk_id = %chunk.id,
                attempt = attempt,
                response_len = response.content.len(),
                response_preview = %&response.content[..response.content.len().min(500)],
                finish_reason = ?response.finish_reason,
                "Raw LLM response received"
            );

            // CRITICAL: Validate response is not empty
            // WHY: Empty LLM responses cause "Invalid JSON: expected value at line 1 column 1" errors
            // This provides actionable error message instead of cryptic JSON parse errors
            let trimmed_response = response.content.trim();
            if trimmed_response.is_empty() {
                let error_msg = format!(
                    "LLM returned EMPTY response. Chunk: {}KB (~{} tokens). \
                    This usually indicates:\n\
                    1. LLM timeout (check Ollama logs: journalctl -u ollama -f)\n\
                    2. Model crashed or OOM (check ollama ps)\n\
                    3. Context window exhausted (reduce chunk_size)\n\
                    4. Network issue with Ollama server\n\
                    Chunk ID: {} | Attempt: {}/{} | Prompt tokens: {}",
                    chunk_size_bytes / 1024,
                    estimated_tokens,
                    chunk.id,
                    attempt,
                    MAX_RETRIES,
                    response.prompt_tokens
                );
                tracing::error!(
                    chunk_id = %chunk.id,
                    attempt = attempt,
                    prompt_tokens = response.prompt_tokens,
                    completion_tokens = response.completion_tokens,
                    "{}",
                    error_msg
                );
                last_error = Some(PipelineError::ExtractionError(error_msg));
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        100 * 2_u64.pow(attempt - 1),
                    ))
                    .await;
                    continue;
                } else {
                    return Err(last_error.unwrap());
                }
            }

            // Check if response was truncated (finish_reason = "length")
            let was_truncated = response
                .finish_reason
                .as_ref()
                .map(|r| r.contains("length"))
                .unwrap_or(false);

            if was_truncated {
                tracing::warn!(
                    attempt = attempt,
                    chunk_id = %chunk.id,
                    current_max_tokens = current_max_tokens,
                    completion_tokens = response.completion_tokens,
                    "LLM response truncated (finish_reason=length), will retry with higher limit"
                );

                // Double max_tokens for retry (up to 32768 max)
                if attempt < MAX_RETRIES && current_max_tokens < 32768 {
                    current_max_tokens = (current_max_tokens * 2).min(32768);
                    last_error = Some(PipelineError::ExtractionError(format!(
                        "JSON truncated at {} tokens. Retrying with {} tokens (attempt {}/{})",
                        response.completion_tokens,
                        current_max_tokens,
                        attempt + 1,
                        MAX_RETRIES
                    )));
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        100 * 2_u64.pow(attempt - 1),
                    ))
                    .await;
                    continue;
                } else {
                    return Err(PipelineError::ExtractionError(format!(
                        "JSON still truncated after {} retries. Max tokens reached: {}. \
                        Document may be too large for entity extraction. \
                        Suggestion: Split document into smaller chunks or use a model with larger context window.",
                        MAX_RETRIES,
                        current_max_tokens
                    )));
                }
            }

            // Parse response using hybrid parser (with built-in fallbacks)
            match self.parser.parse(&response.content, &chunk.id) {
                Ok(mut result) => {
                    // Add token usage from response
                    result.input_tokens = response.prompt_tokens;
                    result.output_tokens = response.completion_tokens;
                    result.extraction_time_ms = start.elapsed().as_millis() as u64;

                    // Add source chunk line info to metadata
                    result
                        .metadata
                        .insert("extractor".to_string(), serde_json::json!("sota"));
                    result
                        .metadata
                        .insert("language".to_string(), serde_json::json!(self.language));
                    result
                        .metadata
                        .insert("model".to_string(), serde_json::json!(response.model));
                    result
                        .metadata
                        .insert("parse_attempts".to_string(), serde_json::json!(attempt));
                    result.metadata.insert(
                        "max_tokens_used".to_string(),
                        serde_json::json!(current_max_tokens),
                    );

                    if attempt > 1 {
                        tracing::info!(
                            attempt = attempt,
                            chunk_id = %chunk.id,
                            entities = result.entities.len(),
                            "Extraction succeeded after retry"
                        );
                    }

                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = e.to_string().to_lowercase();
                    let is_json_truncation = error_msg.contains("eof")
                        || error_msg.contains("unexpected end")
                        || error_msg.contains("unclosed");

                    tracing::warn!(
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        chunk_id = %chunk.id,
                        is_json_truncation = is_json_truncation,
                        current_max_tokens = current_max_tokens,
                        response_preview = %&response.content[..response.content.len().min(200)],
                        "Parsing failed - malformed LLM response"
                    );

                    // If likely JSON truncation, increase max_tokens and retry
                    if is_json_truncation && attempt < MAX_RETRIES && current_max_tokens < 32768 {
                        current_max_tokens = (current_max_tokens * 2).min(32768);
                        tracing::info!(
                            chunk_id = %chunk.id,
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            new_max_tokens = current_max_tokens,
                            "Detected JSON truncation, increasing max_tokens and retrying"
                        );
                        last_error = Some(e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    }

                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        // Exponential backoff: 100ms, 200ms, 400ms
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * 2_u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            PipelineError::ExtractionError("Unknown extraction error after retries".to_string())
        }))
    }

    fn name(&self) -> &str {
        "sota"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
}
