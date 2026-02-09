//! Reranking functionality for improved retrieval quality.
//!
//! This module provides reranking capabilities to improve search result relevance
//! by scoring documents against a query using specialized reranking models.
//!
//! ## Implements
//!
//! - **FEAT0774**: Reranking for improved retrieval
//! - **FEAT0775**: Multi-provider reranker support
//! - **FEAT0776**: BM25 keyword fallback scoring
//!
//! ## Enforces
//!
//! - **BR0774**: Top-k results after reranking
//! - **BR0775**: Fallback to BM25 if reranker unavailable
//!
//! Supports multiple reranking providers:
//! - Jina AI Reranker
//! - Cohere Rerank
//! - Aliyun DashScope
//! - OpenAI-compatible rerankers
//!
//! Based on LightRAG's rerank.py implementation.

use crate::error::{LlmError, Result};
use async_trait::async_trait;
use reqwest::Client;
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};
use unicode_normalization::UnicodeNormalization;

/// Result from reranking a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Index of the document in the original list.
    pub index: usize,
    /// Relevance score (higher is more relevant).
    pub relevance_score: f64,
}

/// Configuration for a reranker.
#[derive(Debug, Clone)]
pub struct RerankConfig {
    /// Model name to use.
    pub model: String,
    /// Base URL for the reranker API.
    pub base_url: String,
    /// API key for authentication.
    pub api_key: Option<String>,
    /// Maximum number of results to return.
    pub top_n: Option<usize>,
    /// Request timeout.
    pub timeout: Duration,
    /// Enable document chunking for long documents.
    pub enable_chunking: bool,
    /// Maximum tokens per document for chunking.
    pub max_tokens_per_doc: usize,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            model: "jina-reranker-v2-base-multilingual".to_string(),
            base_url: "https://api.jina.ai/v1/rerank".to_string(),
            api_key: None,
            top_n: None,
            timeout: Duration::from_secs(30),
            enable_chunking: false,
            max_tokens_per_doc: 480,
        }
    }
}

impl RerankConfig {
    /// Create a new Jina reranker config.
    pub fn jina(api_key: impl Into<String>) -> Self {
        Self {
            model: "jina-reranker-v2-base-multilingual".to_string(),
            base_url: "https://api.jina.ai/v1/rerank".to_string(),
            api_key: Some(api_key.into()),
            ..Default::default()
        }
    }

    /// Create a new Cohere reranker config.
    pub fn cohere(api_key: impl Into<String>) -> Self {
        Self {
            model: "rerank-v3.5".to_string(),
            base_url: "https://api.cohere.com/v2/rerank".to_string(),
            api_key: Some(api_key.into()),
            max_tokens_per_doc: 4096,
            ..Default::default()
        }
    }

    /// Create a new Aliyun DashScope reranker config.
    pub fn aliyun(api_key: impl Into<String>) -> Self {
        Self {
            model: "gte-rerank-v2".to_string(),
            base_url:
                "https://dashscope.aliyuncs.com/api/v1/services/rerank/text-rerank/text-rerank"
                    .to_string(),
            api_key: Some(api_key.into()),
            ..Default::default()
        }
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the top N results to return.
    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = Some(top_n);
        self
    }

    /// Enable document chunking.
    pub fn with_chunking(mut self, enable: bool) -> Self {
        self.enable_chunking = enable;
        self
    }

    /// Set max tokens per document for chunking.
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens_per_doc = max_tokens;
        self
    }
}

/// Strategy for aggregating chunk scores.
#[derive(Debug, Clone, Copy, Default)]
pub enum ScoreAggregation {
    /// Use the maximum score from all chunks.
    #[default]
    Max,
    /// Use the mean of all chunk scores.
    Mean,
    /// Use the score from the first chunk.
    First,
}

/// Trait for reranking providers.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Get the name of this reranker.
    fn name(&self) -> &str;

    /// Get the model being used.
    fn model(&self) -> &str;

    /// Rerank documents based on relevance to a query.
    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>>;

    /// Rerank with documents as string slices.
    async fn rerank_str(
        &self,
        query: &str,
        documents: &[&str],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        let docs: Vec<String> = documents.iter().map(|s| s.to_string()).collect();
        self.rerank(query, &docs, top_n).await
    }
}

/// HTTP-based reranker that supports multiple providers.
pub struct HttpReranker {
    client: Client,
    config: RerankConfig,
    /// Response format for parsing results.
    response_format: ResponseFormat,
    /// Request format for building payloads.
    request_format: RequestFormat,
}

#[derive(Debug, Clone, Copy)]
enum ResponseFormat {
    /// Standard format: {"results": [{"index": 0, "relevance_score": 0.9}]}
    Standard,
    /// Aliyun format: {"output": {"results": [...]}}
    Aliyun,
}

#[derive(Debug, Clone, Copy)]
enum RequestFormat {
    /// Standard format: {"query": "...", "documents": [...]}
    Standard,
    /// Aliyun format: {"input": {"query": "...", "documents": [...]}}
    Aliyun,
}

impl HttpReranker {
    /// Create a new HTTP reranker with the given config.
    pub fn new(config: RerankConfig) -> Self {
        let (response_format, request_format) = Self::detect_format(&config.base_url);

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            config,
            response_format,
            request_format,
        }
    }

    /// Create a Jina reranker.
    pub fn jina(api_key: impl Into<String>) -> Self {
        Self::new(RerankConfig::jina(api_key))
    }

    /// Create a Cohere reranker.
    pub fn cohere(api_key: impl Into<String>) -> Self {
        Self::new(RerankConfig::cohere(api_key))
    }

    /// Create an Aliyun reranker.
    pub fn aliyun(api_key: impl Into<String>) -> Self {
        let config = RerankConfig::aliyun(api_key);
        Self {
            client: Client::builder()
                .timeout(config.timeout)
                .build()
                .expect("Failed to build HTTP client"),
            config,
            response_format: ResponseFormat::Aliyun,
            request_format: RequestFormat::Aliyun,
        }
    }

    fn detect_format(base_url: &str) -> (ResponseFormat, RequestFormat) {
        if base_url.contains("dashscope.aliyuncs.com") {
            (ResponseFormat::Aliyun, RequestFormat::Aliyun)
        } else {
            (ResponseFormat::Standard, RequestFormat::Standard)
        }
    }

    fn build_request(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> serde_json::Value {
        match self.request_format {
            RequestFormat::Standard => {
                let mut payload = serde_json::json!({
                    "model": self.config.model,
                    "query": query,
                    "documents": documents,
                });
                if let Some(n) = top_n {
                    payload["top_n"] = serde_json::json!(n);
                }
                payload
            }
            RequestFormat::Aliyun => {
                let mut params = serde_json::Map::new();
                if let Some(n) = top_n {
                    params.insert("top_n".to_string(), serde_json::json!(n));
                }
                serde_json::json!({
                    "model": self.config.model,
                    "input": {
                        "query": query,
                        "documents": documents,
                    },
                    "parameters": params,
                })
            }
        }
    }

    fn parse_response(&self, response: serde_json::Value) -> Result<Vec<RerankResult>> {
        let results = match self.response_format {
            ResponseFormat::Standard => response
                .get("results")
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default(),
            ResponseFormat::Aliyun => response
                .get("output")
                .and_then(|o| o.get("results"))
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default(),
        };

        if results.is_empty() {
            warn!("Rerank API returned empty results");
            return Ok(vec![]);
        }

        let mut rerank_results = Vec::with_capacity(results.len());
        for result in results {
            let index = result
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or_else(|| LlmError::Unknown("Missing index in rerank result".to_string()))?
                as usize;
            let score = result
                .get("relevance_score")
                .and_then(|s| s.as_f64())
                .ok_or_else(|| {
                    LlmError::Unknown("Missing relevance_score in rerank result".to_string())
                })?;

            rerank_results.push(RerankResult {
                index,
                relevance_score: score,
            });
        }

        // Sort by relevance score descending
        rerank_results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(rerank_results)
    }

    /// Chunk documents that exceed the token limit.
    fn chunk_documents(&self, documents: &[String]) -> (Vec<String>, Vec<usize>) {
        if !self.config.enable_chunking {
            let indices: Vec<usize> = (0..documents.len()).collect();
            return (documents.to_vec(), indices);
        }

        let max_chars = self.config.max_tokens_per_doc * 4; // Approximate 1 token ≈ 4 chars
        let overlap_chars = 32 * 4; // 32 tokens overlap

        let mut chunked = Vec::new();
        let mut indices = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            if doc.len() <= max_chars {
                chunked.push(doc.clone());
                indices.push(idx);
            } else {
                // Split into overlapping chunks
                let mut start = 0;
                while start < doc.len() {
                    let end = (start + max_chars).min(doc.len());
                    let chunk = doc[start..end].to_string();
                    chunked.push(chunk);
                    indices.push(idx);

                    if end >= doc.len() {
                        break;
                    }
                    start = end.saturating_sub(overlap_chars);
                }
            }
        }

        debug!(
            "Chunked {} documents into {} chunks",
            documents.len(),
            chunked.len()
        );
        (chunked, indices)
    }

    /// Aggregate chunk scores back to original documents.
    fn aggregate_scores(
        &self,
        chunk_results: Vec<RerankResult>,
        doc_indices: &[usize],
        num_docs: usize,
        aggregation: ScoreAggregation,
    ) -> Vec<RerankResult> {
        let mut doc_scores: HashMap<usize, Vec<f64>> = HashMap::new();
        for i in 0..num_docs {
            doc_scores.insert(i, Vec::new());
        }

        for result in chunk_results {
            if result.index < doc_indices.len() {
                let original_idx = doc_indices[result.index];
                if let Some(scores) = doc_scores.get_mut(&original_idx) {
                    scores.push(result.relevance_score);
                }
            }
        }

        let mut aggregated: Vec<RerankResult> = doc_scores
            .into_iter()
            .filter(|(_, scores)| !scores.is_empty())
            .map(|(idx, scores)| {
                let final_score = match aggregation {
                    ScoreAggregation::Max => {
                        scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                    }
                    ScoreAggregation::Mean => scores.iter().sum::<f64>() / scores.len() as f64,
                    ScoreAggregation::First => scores[0],
                };
                RerankResult {
                    index: idx,
                    relevance_score: final_score,
                }
            })
            .collect();

        aggregated.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        aggregated
    }
}

#[async_trait]
impl Reranker for HttpReranker {
    fn name(&self) -> &str {
        if self.config.base_url.contains("jina.ai") {
            "jina"
        } else if self.config.base_url.contains("cohere.com") {
            "cohere"
        } else if self.config.base_url.contains("aliyuncs.com") {
            "aliyun"
        } else {
            "http"
        }
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Handle chunking
        let (chunked_docs, doc_indices) = self.chunk_documents(documents);
        let original_top_n = top_n;

        // When chunking, disable top_n at API level to get all chunk scores
        let api_top_n = if self.config.enable_chunking {
            None
        } else {
            top_n
        };

        let payload = self.build_request(query, &chunked_docs, api_top_n);

        debug!(
            "Rerank request: {} documents, model: {}",
            chunked_docs.len(),
            self.config.model
        );

        let mut request = self
            .client
            .post(&self.config.base_url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&payload)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Rerank API error ({}): {}",
                status.as_u16(),
                error_text
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::Unknown(format!("Failed to parse rerank response: {}", e)))?;

        let mut results = self.parse_response(response_json)?;

        // Aggregate chunk scores if chunking was enabled
        if self.config.enable_chunking && chunked_docs.len() != documents.len() {
            results = self.aggregate_scores(
                results,
                &doc_indices,
                documents.len(),
                ScoreAggregation::Max,
            );
        }

        // Apply top_n limit at document level
        if let Some(n) = original_top_n {
            results.truncate(n);
        }

        Ok(results)
    }
}

/// Term overlap reranker using simple Jaccard-like scoring.
///
/// WHY this reranker: Provides a fast, no-external-dependency reranker for:
/// - Testing and development (no API keys required)
/// - Fallback when BM25 isn't needed
/// - Simple use cases where full BM25 is overkill
///
/// ## Algorithm
///
/// Scores documents by computing the fraction of query terms that appear in the document:
/// `score = |query_terms ∩ doc_terms| / |query_terms|`
///
/// ## Limitations
///
/// - No IDF weighting (rare terms not prioritized)
/// - No term frequency consideration (TF=1 vs TF=10 same score)
/// - No length normalization
///
/// For production use, prefer `BM25Reranker` which addresses all these limitations.
///
/// ## Backward Compatibility
///
/// Previously named `MockReranker`. The type alias `MockReranker = TermOverlapReranker`
/// is provided for backward compatibility.
pub struct TermOverlapReranker {
    model: String,
}

impl TermOverlapReranker {
    /// Create a new term overlap reranker.
    pub fn new() -> Self {
        Self {
            model: "term-overlap-reranker".to_string(),
        }
    }
}

impl Default for TermOverlapReranker {
    fn default() -> Self {
        Self::new()
    }
}

/// Backward compatibility alias for `TermOverlapReranker`.
///
/// Deprecated: Use `TermOverlapReranker` for new code.
pub type MockReranker = TermOverlapReranker;

#[async_trait]
impl Reranker for TermOverlapReranker {
    fn name(&self) -> &str {
        "term-overlap"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        // Score based on query term overlap (Jaccard-like metric)
        let query_lower = query.to_lowercase();
        let query_terms: std::collections::HashSet<String> = query_lower
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut results: Vec<RerankResult> = documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                let doc_lower = doc.to_lowercase();
                let doc_terms: std::collections::HashSet<String> = doc_lower
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let overlap = query_terms.intersection(&doc_terms).count();
                let max_terms = query_terms.len().max(1);
                let score = overlap as f64 / max_terms as f64;

                RerankResult {
                    index: idx,
                    relevance_score: score,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply top_n
        if let Some(n) = top_n {
            results.truncate(n);
        }

        Ok(results)
    }
}

/// BM25 reranker for high-quality text-based reranking.
///
/// WHY BM25: Industry-standard ranking algorithm used by Elasticsearch, Lucene, etc.
/// Advantages over simple term overlap:
/// - IDF weighting: rare terms score higher (e.g., "ENVY" vs "the")
/// - Term frequency saturation: diminishing returns for repeated terms
/// - Length normalization: long docs don't dominate short focused ones
///
/// This implementation is SOTA-compliant based on the Okapi BM25 algorithm
/// (Robertson et al.) with optional BM25+ extension for better long document handling.
///
/// ## Parameters
///
/// - `k1` ∈ [1.2, 2.0]: Term frequency saturation (higher = more TF influence)
/// - `b` ∈ [0, 1]: Length normalization (0 = no normalization, 1 = full normalization)
/// - `delta` ≥ 0: BM25+ extension parameter (0 = standard BM25, 1.0 = BM25+)
///
/// ## IDF Formula (SOTA)
///
/// `IDF(q) = ln((N - n(q) + 0.5) / (n(q) + 0.5) + 1)`
///
/// The `+1` inside the logarithm ensures IDF is always non-negative.
///
/// ## Scoring Formula
///
/// Standard BM25:
/// `score = Σ IDF(q) × f(q,D)×(k1+1) / (f(q,D) + k1×(1-b+b×|D|/avgdl))`
///
/// BM25+ (when delta > 0):
/// `score = Σ IDF(q) × (f(q,D)×(k1+1) / (f(q,D) + k1×(1-b+b×|D|/avgdl)) + delta)`
///
/// ## References
///
/// - Wikipedia: <https://en.wikipedia.org/wiki/Okapi_BM25>
/// - Robertson, S., Zaragoza, H. (2009). The Probabilistic Relevance Framework: BM25 and Beyond
/// - Lv, Y., Zhai, C. (2011). Lower-Bounding Term Frequency Normalization (BM25+)
///
/// # Example
///
/// ```
/// use edgequake_llm::reranker::BM25Reranker;
///
/// // Create a BM25 reranker with default parameters
/// let reranker = BM25Reranker::new();
///
/// // Or use a preset for specific use cases
/// let rag_reranker = BM25Reranker::for_rag();
/// ```
pub struct BM25Reranker {
    /// Term frequency saturation parameter (k1).
    /// WHY: Controls how quickly term frequency saturates. Higher values give
    /// more weight to repeated terms. Standard range: [1.2, 2.0].
    k1: f64,
    /// Length normalization parameter (b).
    /// WHY: Controls document length penalty. b=0 means no length normalization,
    /// b=1 means full normalization. Standard value: 0.75.
    b: f64,
    /// BM25+ delta parameter for long document handling.
    /// WHY: Standard BM25 can penalize long documents unfairly even when they
    /// contain all query terms. Adding delta ensures a minimum score contribution.
    /// Set to 0.0 for standard BM25, 1.0 for BM25+.
    delta: f64,
    /// Phrase boost parameter for adjacent term matching.
    /// WHY: Standard BM25 treats "knowledge graph" and "graph knowledge" the same.
    /// Phrase boosting adds a bonus when query terms appear in order.
    /// Set to 0.0 to disable, 0.5-1.0 for moderate boost.
    phrase_boost: f64,
    /// Model name for trait compliance
    model: String,
    /// Tokenizer configuration for enhanced text processing.
    tokenizer_config: TokenizerConfig,
}

/// Configuration for the BM25 tokenizer.
///
/// WHY: Provides fine-grained control over text processing to improve
/// search relevance for different languages and use cases.
#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    /// Enable Porter2 stemming for English text.
    /// WHY: Stemming improves recall by matching morphological variants
    /// (e.g., "running" → "run", "fruitlessly" → "fruitless").
    pub enable_stemming: bool,
    /// Stemmer algorithm to use (defaults to English).
    pub stemmer_algorithm: Algorithm,
    /// Enable stop word filtering.
    /// WHY: Stop words like "the", "a", "is" add noise without semantic value.
    pub enable_stop_words: bool,
    /// Minimum token length (tokens shorter than this are filtered).
    /// WHY: Single-character tokens are usually noise, but we keep 2+ for CJK.
    pub min_token_length: usize,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            enable_stemming: true,
            stemmer_algorithm: Algorithm::English,
            enable_stop_words: true,
            min_token_length: 2,
        }
    }
}

impl TokenizerConfig {
    /// Create a minimal tokenizer without stemming or stop words (backward compatible).
    pub fn minimal() -> Self {
        Self {
            enable_stemming: false,
            stemmer_algorithm: Algorithm::English,
            enable_stop_words: false,
            min_token_length: 2,
        }
    }

    /// Create an enhanced tokenizer with stemming and stop words.
    pub fn enhanced() -> Self {
        Self::default()
    }

    /// Create a French tokenizer.
    pub fn french() -> Self {
        Self {
            enable_stemming: true,
            stemmer_algorithm: Algorithm::French,
            enable_stop_words: true,
            min_token_length: 2,
        }
    }
}

/// Common English stop words.
/// WHY: These high-frequency words don't carry semantic meaning and dilute IDF.
const ENGLISH_STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "or", "that", "the", "to", "was", "were", "will", "with", "this", "but",
    "they", "have", "had", "what", "when", "where", "who", "which", "you", "your", "we", "our",
    "can", "all", "there", "their", "been", "would", "could", "should", "may", "might", "must",
    "do", "does", "did", "if", "not", "no", "so", "up", "out", "just", "than", "then", "too",
    "very", "also",
];

impl BM25Reranker {
    /// Create a new BM25 reranker with default parameters.
    ///
    /// Defaults: k1 = 1.5, b = 0.75, delta = 0.0 (standard BM25)
    /// Uses minimal tokenizer (backward compatible, no stemming).
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_llm::reranker::BM25Reranker;
    ///
    /// let reranker = BM25Reranker::new();
    /// ```
    pub fn new() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 0.0,        // Standard BM25, not BM25+
            phrase_boost: 0.0, // No phrase boosting by default
            model: "bm25-reranker".to_string(),
            tokenizer_config: TokenizerConfig::minimal(), // Backward compatible
        }
    }

    /// Create a new BM25 reranker with enhanced tokenization.
    ///
    /// This version includes stemming and stop word filtering for
    /// improved search relevance.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_llm::reranker::BM25Reranker;
    ///
    /// // Enhanced reranker with stemming (running → run)
    /// let reranker = BM25Reranker::new_enhanced();
    /// ```
    pub fn new_enhanced() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 0.0,
            phrase_boost: 0.0,
            model: "bm25-enhanced-reranker".to_string(),
            tokenizer_config: TokenizerConfig::enhanced(),
        }
    }

    /// Create a BM25+ reranker with delta = 1.0 for better long document handling.
    ///
    /// BM25+ adds a delta parameter to prevent long documents from being
    /// penalized too heavily, ensuring each matching term contributes at least
    /// delta to the score.
    pub fn bm25_plus() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 1.0, // BM25+ extension
            phrase_boost: 0.0,
            model: "bm25-plus-reranker".to_string(),
            tokenizer_config: TokenizerConfig::minimal(),
        }
    }

    // =========================================================================
    // Domain-Specific Presets (OODA Loop 5)
    // =========================================================================

    /// Create reranker optimized for short documents (tweets, titles, snippets).
    ///
    /// WHY these parameters:
    /// - `k1=1.2`: Lower saturation for short content where each term matters
    /// - `b=0.3`: Reduced length normalization (short docs shouldn't be penalized)
    /// - `delta=0`: Standard BM25 sufficient for short content
    pub fn for_short_docs() -> Self {
        Self {
            k1: 1.2,
            b: 0.3,
            delta: 0.0,
            phrase_boost: 0.0,
            model: "bm25-short-docs".to_string(),
            tokenizer_config: TokenizerConfig::enhanced(),
        }
    }

    /// Create reranker optimized for long documents (papers, articles, books).
    ///
    /// WHY these parameters:
    /// - `k1=1.5`: Standard saturation
    /// - `b=0.75`: Full length normalization
    /// - `delta=1.0`: BM25+ extension prevents over-penalizing long relevant docs
    pub fn for_long_docs() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 1.0,
            phrase_boost: 0.0,
            model: "bm25-long-docs".to_string(),
            tokenizer_config: TokenizerConfig::enhanced(),
        }
    }

    /// Create reranker optimized for technical content (code, APIs, docs).
    ///
    /// WHY these parameters:
    /// - `k1=2.0`: Higher saturation gives more weight to repeated terms
    /// - `b=0.5`: Moderate length normalization
    /// - No stemming: Technical terms should match exactly
    pub fn for_technical() -> Self {
        Self {
            k1: 2.0,
            b: 0.5,
            delta: 0.0,
            phrase_boost: 0.0,
            model: "bm25-technical".to_string(),
            tokenizer_config: TokenizerConfig::minimal(), // No stemming for exact matches
        }
    }

    /// Create reranker optimized for RAG/knowledge graph queries.
    ///
    /// WHY these parameters:
    /// - `k1=1.5`: Balanced saturation
    /// - `b=0.75`: Standard length normalization
    /// - `delta=0.5`: Mild BM25+ for mixed-length chunks
    /// - Enhanced tokenization for semantic matching
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_llm::reranker::BM25Reranker;
    ///
    /// // Best for EdgeQuake knowledge graph queries
    /// let reranker = BM25Reranker::for_rag();
    /// ```
    pub fn for_rag() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 0.5,
            phrase_boost: 0.3, // Moderate phrase boosting for RAG queries
            model: "bm25-rag".to_string(),
            tokenizer_config: TokenizerConfig::enhanced(),
        }
    }

    /// Create with custom parameters.
    ///
    /// # Arguments
    /// - `k1`: Term frequency saturation [0.0, 3.0]
    /// - `b`: Length normalization [0.0, 1.0]
    pub fn with_params(k1: f64, b: f64) -> Self {
        Self {
            k1: k1.clamp(0.0, 3.0),
            b: b.clamp(0.0, 1.0),
            delta: 0.0,
            phrase_boost: 0.0,
            model: "bm25-reranker".to_string(),
            tokenizer_config: TokenizerConfig::minimal(),
        }
    }

    /// Create with full custom parameters including BM25+ delta.
    ///
    /// # Arguments
    /// - `k1`: Term frequency saturation [0.0, 3.0]
    /// - `b`: Length normalization [0.0, 1.0]
    /// - `delta`: BM25+ extension parameter (0 = standard, ≥1.0 recommended for BM25+)
    pub fn with_full_params(k1: f64, b: f64, delta: f64) -> Self {
        Self {
            k1: k1.clamp(0.0, 3.0),
            b: b.clamp(0.0, 1.0),
            delta: delta.max(0.0),
            phrase_boost: 0.0,
            model: if delta > 0.0 {
                "bm25-plus-reranker".to_string()
            } else {
                "bm25-reranker".to_string()
            },
            tokenizer_config: TokenizerConfig::minimal(),
        }
    }

    /// Create with custom tokenizer configuration.
    ///
    /// This allows fine-tuning tokenization for specific languages or use cases.
    pub fn with_tokenizer_config(mut self, config: TokenizerConfig) -> Self {
        self.tokenizer_config = config;
        self
    }

    /// Set phrase boost factor.
    ///
    /// WHY: Phrase boosting rewards documents where query terms appear in order.
    /// This helps distinguish "knowledge graph" from "graph of knowledge".
    ///
    /// # Arguments
    /// - `boost`: Phrase boost factor [0.0, 2.0]. 0.0 disables, 0.5-1.0 recommended.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_llm::reranker::BM25Reranker;
    ///
    /// // Add phrase boosting to any reranker
    /// let reranker = BM25Reranker::new_enhanced()
    ///     .with_phrase_boost(0.5);
    /// ```
    pub fn with_phrase_boost(mut self, boost: f64) -> Self {
        self.phrase_boost = boost.clamp(0.0, 2.0);
        self
    }

    /// Create a phrase-boosted reranker for semantic queries.
    ///
    /// WHY these parameters:
    /// - Enhanced tokenization for semantic matching
    /// - phrase_boost=0.5 for moderate phrase preference
    /// - Standard BM25+ parameters for balanced scoring
    pub fn for_semantic() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 0.5,
            phrase_boost: 0.5,
            model: "bm25-semantic".to_string(),
            tokenizer_config: TokenizerConfig::enhanced(),
        }
    }

    /// Check if a word is a stop word.
    fn is_stop_word(word: &str) -> bool {
        ENGLISH_STOP_WORDS.binary_search(&word).is_ok()
    }

    /// Tokenize text using the configured tokenizer settings.
    ///
    /// WHY: Enhanced tokenization improves search relevance by:
    /// 1. Unicode normalization: Handles accents consistently (café → cafe)
    /// 2. Stemming: Matches morphological variants (running → run)
    /// 3. Stop word removal: Reduces noise from high-frequency words
    fn tokenize_with_config(&self, text: &str) -> Vec<String> {
        // WHY: NFKD decomposition separates base characters from combining marks,
        // allowing us to strip accents universally instead of hardcoded mappings.
        let normalized: String = text
            .to_lowercase()
            .nfkd()
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            .collect();

        let tokens: Vec<String> = normalized
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() >= self.tokenizer_config.min_token_length)
            .map(|s| s.to_string())
            .collect();

        // Apply stop word filtering if enabled
        let filtered: Vec<String> = if self.tokenizer_config.enable_stop_words {
            tokens
                .into_iter()
                .filter(|t| !Self::is_stop_word(t))
                .collect()
        } else {
            tokens
        };

        // Apply stemming if enabled
        if self.tokenizer_config.enable_stemming {
            let stemmer = Stemmer::create(self.tokenizer_config.stemmer_algorithm);
            filtered
                .into_iter()
                .map(|t| stemmer.stem(&t).to_string())
                .collect()
        } else {
            filtered
        }
    }

    /// Tokenize text into lowercase words, Unicode-normalized.
    /// WHY: Backward-compatible tokenization for existing tests.
    fn tokenize(text: &str) -> Vec<String> {
        // WHY: Use NFKD normalization instead of hardcoded accent mappings.
        // This handles all Unicode accents, not just French.
        let normalized: String = text
            .to_lowercase()
            .nfkd()
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            .collect();

        normalized
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// Compute IDF for a term across documents.
    ///
    /// WHY this formula: Robertson et al. introduced the +1 inside ln() to ensure
    /// IDF is always non-negative, preventing negative scores for very common terms.
    ///
    /// Formula: ln((N - n(q) + 0.5) / (n(q) + 0.5) + 1)
    ///
    /// Where:
    /// - N = total number of documents
    /// - n(q) = number of documents containing term q
    #[allow(dead_code)] // Keep for backward compatibility and testing
    fn compute_idf(term: &str, doc_terms_list: &[Vec<String>]) -> f64 {
        let n = doc_terms_list.len() as f64;
        let containing_docs = doc_terms_list
            .iter()
            .filter(|terms| terms.contains(&term.to_string()))
            .count() as f64;

        Self::compute_idf_from_df(n, containing_docs)
    }

    /// Compute IDF from pre-computed document frequency.
    ///
    /// WHY: O(1) computation instead of O(n) scan per term.
    /// Used with compute_document_frequencies() for batch processing.
    #[inline]
    fn compute_idf_from_df(n: f64, df: f64) -> f64 {
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    /// Build document frequency map for all terms in corpus.
    ///
    /// WHY: Pre-computing DF allows O(1) IDF lookups instead of O(n) per term.
    /// For k query terms × n documents, this reduces complexity from O(k×n) to O(n + k).
    ///
    /// Returns: HashMap<term, count of documents containing term>
    fn compute_document_frequencies(doc_terms_list: &[Vec<String>]) -> HashMap<String, usize> {
        use std::collections::HashSet;

        let mut df_map: HashMap<String, usize> = HashMap::new();

        for doc_terms in doc_terms_list {
            // WHY: Use HashSet to count each term only once per document.
            // This handles documents with repeated terms correctly.
            let unique_terms: HashSet<&String> = doc_terms.iter().collect();
            for term in unique_terms {
                *df_map.entry(term.clone()).or_insert(0) += 1;
            }
        }

        df_map
    }

    /// Compute BM25/BM25+ score for a single document.
    ///
    /// WHY this implementation:
    /// - Standard BM25 formula with TF saturation and length normalization
    /// - BM25+ extension (delta > 0) addresses the "long document penalty" issue
    ///   where long documents are unfairly penalized even when containing query terms
    ///
    /// Formula (BM25):
    /// `score = Σ IDF(q) × f(q,D)×(k1+1) / (f(q,D) + k1×(1-b+b×|D|/avgdl))`
    ///
    /// Formula (BM25+):
    /// `score = Σ IDF(q) × (f(q,D)×(k1+1) / (f(q,D) + k1×(1-b+b×|D|/avgdl)) + delta)`
    /// Compute phrase match bonus for adjacent query term pairs.
    ///
    /// WHY: Standard BM25 treats "knowledge graph" and "graph knowledge" the same.
    /// This method adds a bonus when consecutive query terms appear adjacent
    /// in the document, rewarding phrase matches.
    ///
    /// # Algorithm
    /// For each consecutive pair of query terms (a, b):
    /// - Search for occurrences of "a" followed immediately by "b" in the document
    /// - Count matches and return normalized bonus
    ///
    /// # Complexity
    /// O(q × d) where q = query length, d = document length
    fn compute_phrase_bonus(&self, query_terms: &[String], doc_terms: &[String]) -> f64 {
        if query_terms.len() < 2 || doc_terms.len() < 2 {
            return 0.0;
        }

        let mut phrase_matches = 0;
        let total_pairs = query_terms.len().saturating_sub(1);

        // Check each consecutive query term pair
        for window in query_terms.windows(2) {
            let (term_a, term_b) = (&window[0], &window[1]);

            // Search for adjacent occurrences in document
            for doc_window in doc_terms.windows(2) {
                if &doc_window[0] == term_a && &doc_window[1] == term_b {
                    phrase_matches += 1;
                    break; // Count each phrase pair once per query pair
                }
            }
        }

        // Normalize by number of query pairs to keep bonus in [0, 1] range
        // WHY: Ensures phrase_boost parameter has consistent effect regardless of query length
        phrase_matches as f64 / total_pairs.max(1) as f64
    }

    fn compute_bm25_score(
        &self,
        query_terms: &[String],
        doc_terms: &[String],
        avgdl: f64,
        idf_cache: &std::collections::HashMap<String, f64>,
    ) -> f64 {
        let doc_len = doc_terms.len() as f64;
        let length_norm = 1.0 - self.b + self.b * (doc_len / avgdl);

        let mut score = 0.0;
        for term in query_terms {
            let tf = doc_terms.iter().filter(|t| t == &term).count() as f64;
            if tf > 0.0 {
                let idf = idf_cache.get(term).copied().unwrap_or(0.0);
                let tf_component = (tf * (self.k1 + 1.0)) / (tf + self.k1 * length_norm);
                // BM25+: Add delta to ensure minimum contribution per matching term
                // WHY: Prevents long documents from being penalized too heavily
                score += idf * (tf_component + self.delta);
            }
        }
        score
    }
}

impl Default for BM25Reranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for BM25Reranker {
    fn name(&self) -> &str {
        "bm25"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // WHY: Use instance method for tokenization when enhanced config is active,
        // otherwise use static method for backward compatibility.
        let query_terms =
            if self.tokenizer_config.enable_stemming || self.tokenizer_config.enable_stop_words {
                self.tokenize_with_config(query)
            } else {
                Self::tokenize(query)
            };

        if query_terms.is_empty() {
            // Fall back to simple ordering if query is empty
            let results: Vec<RerankResult> = documents
                .iter()
                .enumerate()
                .map(|(idx, _)| RerankResult {
                    index: idx,
                    relevance_score: 0.0,
                })
                .collect();
            return Ok(results);
        }

        // WHY: Tokenize documents with same config as query for consistency
        let doc_terms_list: Vec<Vec<String>> =
            if self.tokenizer_config.enable_stemming || self.tokenizer_config.enable_stop_words {
                documents
                    .iter()
                    .map(|d| self.tokenize_with_config(d))
                    .collect()
            } else {
                documents.iter().map(|d| Self::tokenize(d)).collect()
            };

        // Compute average document length
        let avgdl = doc_terms_list.iter().map(|d| d.len()).sum::<usize>() as f64
            / doc_terms_list.len().max(1) as f64;
        let avgdl = avgdl.max(1.0); // Avoid division by zero

        // WHY: Pre-compute document frequency (DF) map for O(1) IDF lookups.
        // This changes IDF computation from O(n×m) to O(1) per term.
        // For a corpus of 1000 docs with 50 terms each, this is ~50x faster.
        let df_map = Self::compute_document_frequencies(&doc_terms_list);
        let n = doc_terms_list.len() as f64;

        // Pre-compute IDF for all query terms using DF map
        let mut idf_cache = std::collections::HashMap::new();
        for term in &query_terms {
            let df = df_map.get(term).copied().unwrap_or(0) as f64;
            let idf = Self::compute_idf_from_df(n, df);
            idf_cache.insert(term.clone(), idf);
        }

        // Score each document
        let mut results: Vec<RerankResult> = doc_terms_list
            .iter()
            .enumerate()
            .map(|(idx, doc_terms)| {
                let bm25_score =
                    self.compute_bm25_score(&query_terms, doc_terms, avgdl, &idf_cache);

                // WHY: Phrase boosting adds bonus for documents where query terms
                // appear in order. This helps distinguish "knowledge graph" from
                // "graph of knowledge" which standard BM25 would score equally.
                let phrase_bonus = if self.phrase_boost > 0.0 {
                    self.compute_phrase_bonus(&query_terms, doc_terms)
                } else {
                    0.0
                };

                let final_score = bm25_score + (self.phrase_boost * phrase_bonus);
                RerankResult {
                    index: idx,
                    relevance_score: final_score,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply top_n
        if let Some(n) = top_n {
            results.truncate(n);
        }

        Ok(results)
    }
}

/// Reciprocal Rank Fusion (RRF) reranker for combining multiple ranking signals.
///
/// WHY RRF: Combines rankings from different sources without needing score normalization.
/// Formula: score = Σ 1/(k + rank) for each ranking list
///
/// Use cases:
/// - Combining vector similarity + BM25 rankings
/// - Combining results from multiple queries
/// - Hybrid search scenarios
pub struct RRFReranker {
    /// Ranking constant (higher = lower-ranked docs have more influence)
    k: u32,
    /// Model name for trait compliance
    model: String,
}

impl RRFReranker {
    /// Create a new RRF reranker with default k=60.
    pub fn new() -> Self {
        Self {
            k: 60,
            model: "rrf-reranker".to_string(),
        }
    }

    /// Create with custom k value.
    pub fn with_k(k: u32) -> Self {
        Self {
            k: k.max(1),
            model: "rrf-reranker".to_string(),
        }
    }

    /// Fuse multiple ranked lists using RRF.
    ///
    /// Each inner Vec contains document indices in ranked order (best first).
    pub fn fuse(&self, ranked_lists: &[Vec<usize>], num_docs: usize) -> Vec<RerankResult> {
        let mut scores = vec![0.0f64; num_docs];

        for ranked_list in ranked_lists {
            for (rank, &doc_idx) in ranked_list.iter().enumerate() {
                if doc_idx < num_docs {
                    // RRF formula: 1 / (k + rank + 1), rank is 0-indexed
                    scores[doc_idx] += 1.0 / (self.k as f64 + rank as f64 + 1.0);
                }
            }
        }

        let mut results: Vec<RerankResult> = scores
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > 0.0)
            .map(|(idx, score)| RerankResult {
                index: idx,
                relevance_score: score,
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

impl Default for RRFReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for RRFReranker {
    fn name(&self) -> &str {
        "rrf"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        // RRF alone uses BM25 as the single ranking signal
        // For true RRF, use the fuse() method with multiple ranking sources
        let bm25 = BM25Reranker::new();
        let mut results = bm25.rerank(query, documents, None).await?;

        if let Some(n) = top_n {
            results.truncate(n);
        }

        Ok(results)
    }
}

/// Hybrid reranker combining BM25 with vector similarity boosting.
///
/// This reranker uses BM25 as the base and can optionally incorporate
/// pre-computed vector similarity scores using RRF fusion.
pub struct HybridReranker {
    bm25: BM25Reranker,
    rrf: RRFReranker,
    model: String,
}

impl HybridReranker {
    /// Create a new hybrid reranker.
    pub fn new() -> Self {
        Self {
            bm25: BM25Reranker::new(),
            rrf: RRFReranker::new(),
            model: "hybrid-reranker".to_string(),
        }
    }

    /// Rerank with both text and vector signals.
    ///
    /// Arguments:
    /// - query: The search query
    /// - documents: Document texts
    /// - vector_rankings: Optional pre-sorted indices from vector search (best first)
    /// - top_n: Maximum results to return
    pub async fn rerank_hybrid(
        &self,
        query: &str,
        documents: &[String],
        vector_rankings: Option<Vec<usize>>,
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Get BM25 ranking
        let bm25_results = self.bm25.rerank(query, documents, None).await?;
        let bm25_ranking: Vec<usize> = bm25_results.iter().map(|r| r.index).collect();

        // Combine with vector ranking if provided
        let mut ranked_lists = vec![bm25_ranking];
        if let Some(vec_ranking) = vector_rankings {
            ranked_lists.push(vec_ranking);
        }

        // Use RRF to fuse rankings
        let mut results = self.rrf.fuse(&ranked_lists, documents.len());

        if let Some(n) = top_n {
            results.truncate(n);
        }

        Ok(results)
    }
}

impl Default for HybridReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for HybridReranker {
    fn name(&self) -> &str {
        "hybrid"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        // Without vector rankings, just use BM25
        self.bm25.rerank(query, documents, top_n).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rerank_config_defaults() {
        let config = RerankConfig::default();
        assert_eq!(config.model, "jina-reranker-v2-base-multilingual");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_jina_config() {
        let config = RerankConfig::jina("test-key");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert!(config.base_url.contains("jina.ai"));
    }

    #[test]
    fn test_cohere_config() {
        let config = RerankConfig::cohere("test-key");
        assert!(config.base_url.contains("cohere.com"));
        assert_eq!(config.max_tokens_per_doc, 4096);
    }

    #[test]
    fn test_aliyun_config() {
        let config = RerankConfig::aliyun("test-key");
        assert!(config.base_url.contains("aliyuncs.com"));
    }

    #[tokio::test]
    async fn test_mock_reranker() {
        let reranker = MockReranker::new();
        let query = "capital of France";
        let documents = vec![
            "The capital of France is Paris.".to_string(),
            "Tokyo is the capital of Japan.".to_string(),
            "London is the capital of England.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, Some(2)).await.unwrap();

        assert_eq!(results.len(), 2);
        // First result should be about France (has "capital" and "of")
        assert_eq!(results[0].index, 0);
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_mock_reranker_empty_docs() {
        let reranker = MockReranker::new();
        let results = reranker.rerank("test", &[], None).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_score_aggregation() {
        let reranker = HttpReranker::new(RerankConfig::default());

        let chunk_results = vec![
            RerankResult {
                index: 0,
                relevance_score: 0.9,
            },
            RerankResult {
                index: 1,
                relevance_score: 0.8,
            },
            RerankResult {
                index: 2,
                relevance_score: 0.7,
            },
        ];
        let doc_indices = vec![0, 0, 1]; // Two chunks for doc 0, one for doc 1

        let aggregated =
            reranker.aggregate_scores(chunk_results, &doc_indices, 2, ScoreAggregation::Max);

        assert_eq!(aggregated.len(), 2);
        // Doc 0 should have max(0.9, 0.8) = 0.9
        assert_eq!(aggregated[0].index, 0);
        assert!((aggregated[0].relevance_score - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_chunking() {
        let config = RerankConfig::default()
            .with_chunking(true)
            .with_max_tokens(50); // Small for testing
        let reranker = HttpReranker::new(config);

        let documents = vec![
            "Short document.".to_string(),
            "A".repeat(1000), // Long document that will be chunked
        ];

        let (chunked, indices) = reranker.chunk_documents(&documents);

        // Should have more chunks than original documents
        assert!(chunked.len() > 2);
        // All chunks should map back to original documents
        assert!(indices.iter().all(|&i| i < 2));
    }

    // =========== BM25 Reranker Tests ===========

    #[tokio::test]
    async fn test_bm25_reranker_basic() {
        let reranker = BM25Reranker::new();
        let query = "capital of France";
        let documents = vec![
            "The capital of France is Paris.".to_string(),
            "Tokyo is the capital of Japan.".to_string(),
            "London is the capital of England.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        assert_eq!(results.len(), 3);
        // First result should be about France (has "France" which is unique)
        assert_eq!(results[0].index, 0);
        assert!(results[0].relevance_score > results[1].relevance_score);
    }

    #[tokio::test]
    async fn test_bm25_idf_weighting() {
        let reranker = BM25Reranker::new();
        // Query with a rare term "ENVY" and common term "Peugeot"
        let query = "Peugeot ENVY";
        let documents = vec![
            "The Peugeot 2008 ENVY is a great car.".to_string(),
            "Peugeot makes many cars.".to_string(),
            "Peugeot 208 is also available.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Doc 0 has ENVY (rare) + Peugeot, should score highest
        assert_eq!(results[0].index, 0);
        // ENVY is unique to doc 0, so it should have much higher score
        assert!(results[0].relevance_score > results[1].relevance_score * 1.5);
    }

    #[tokio::test]
    async fn test_bm25_2008_vs_208_precision() {
        // Critical test: "2008" should match "2008" better than "208"
        let reranker = BM25Reranker::new();
        let query = "2008";
        let documents = vec![
            "The Peugeot 208 is a compact car.".to_string(), // index 0
            "The Peugeot 2008 is an SUV.".to_string(),       // index 1
            "The Peugeot 3008 is a larger SUV.".to_string(), // index 2
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // "2008" should be first because it exactly matches the query term
        assert_eq!(results[0].index, 1, "2008 document should be first");
        // "208" should NOT be first (this was the precision bug)
        assert_ne!(results[0].index, 0, "208 document should NOT be first");
    }

    #[tokio::test]
    async fn test_bm25_french_accents() {
        let reranker = BM25Reranker::new();
        let query = "vehicule electrique";
        let documents = vec![
            "Le véhicule électrique est l'avenir.".to_string(),
            "Une voiture classique fonctionne à essence.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should match despite accent differences (normalized)
        assert_eq!(results[0].index, 0);
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_empty_documents() {
        let reranker = BM25Reranker::new();
        let results = reranker.rerank("test", &[], None).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_bm25_empty_query() {
        let reranker = BM25Reranker::new();
        let documents = vec!["Some document.".to_string()];
        let results = reranker.rerank("", &documents, None).await.unwrap();
        // Should return all docs with score 0
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relevance_score, 0.0);
    }

    #[tokio::test]
    async fn test_bm25_top_n() {
        let reranker = BM25Reranker::new();
        let documents = vec![
            "Alpha document.".to_string(),
            "Beta document.".to_string(),
            "Gamma document.".to_string(),
        ];

        let results = reranker
            .rerank("document", &documents, Some(2))
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bm25_tokenization() {
        let tokens = BM25Reranker::tokenize("Hello, World! Test-123 véhicule");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"123".to_string()));
        // Accented should be normalized
        assert!(tokens.contains(&"vehicule".to_string()));
    }

    #[test]
    fn test_bm25_custom_params() {
        let reranker = BM25Reranker::with_params(2.0, 0.5);
        assert_eq!(reranker.k1, 2.0);
        assert_eq!(reranker.b, 0.5);
    }

    // =========== BM25+ Extension Tests ===========

    #[test]
    fn test_bm25_plus_constructor() {
        let reranker = BM25Reranker::bm25_plus();
        assert_eq!(reranker.k1, 1.5);
        assert_eq!(reranker.b, 0.75);
        assert_eq!(reranker.delta, 1.0);
        assert_eq!(reranker.model(), "bm25-plus-reranker");
    }

    #[test]
    fn test_bm25_with_full_params() {
        let reranker = BM25Reranker::with_full_params(1.2, 0.8, 0.5);
        assert_eq!(reranker.k1, 1.2);
        assert_eq!(reranker.b, 0.8);
        assert_eq!(reranker.delta, 0.5);
    }

    // =========== Domain-Specific Preset Tests (OODA Loop 5) ===========

    #[test]
    fn test_for_short_docs_preset() {
        let reranker = BM25Reranker::for_short_docs();
        assert_eq!(reranker.k1, 1.2);
        assert_eq!(reranker.b, 0.3);
        assert_eq!(reranker.delta, 0.0);
        assert_eq!(reranker.model(), "bm25-short-docs");
        assert!(reranker.tokenizer_config.enable_stemming);
    }

    #[test]
    fn test_for_long_docs_preset() {
        let reranker = BM25Reranker::for_long_docs();
        assert_eq!(reranker.k1, 1.5);
        assert_eq!(reranker.b, 0.75);
        assert_eq!(reranker.delta, 1.0); // BM25+ for long docs
        assert_eq!(reranker.model(), "bm25-long-docs");
        assert!(reranker.tokenizer_config.enable_stemming);
    }

    #[test]
    fn test_for_technical_preset() {
        let reranker = BM25Reranker::for_technical();
        assert_eq!(reranker.k1, 2.0);
        assert_eq!(reranker.b, 0.5);
        assert_eq!(reranker.delta, 0.0);
        assert_eq!(reranker.model(), "bm25-technical");
        // Technical preset uses minimal tokenization (no stemming)
        assert!(!reranker.tokenizer_config.enable_stemming);
    }

    #[test]
    fn test_for_rag_preset() {
        let reranker = BM25Reranker::for_rag();
        assert_eq!(reranker.k1, 1.5);
        assert_eq!(reranker.b, 0.75);
        assert_eq!(reranker.delta, 0.5); // Mild BM25+ for mixed chunks
        assert_eq!(reranker.model(), "bm25-rag");
        assert!(reranker.tokenizer_config.enable_stemming);
        assert_eq!(reranker.phrase_boost, 0.3); // RAG has moderate phrase boost
    }

    // =========== Phrase Boosting Tests (OODA Loop 7) ===========

    #[test]
    fn test_for_semantic_preset() {
        let reranker = BM25Reranker::for_semantic();
        assert_eq!(reranker.k1, 1.5);
        assert_eq!(reranker.b, 0.75);
        assert_eq!(reranker.phrase_boost, 0.5);
        assert_eq!(reranker.model(), "bm25-semantic");
    }

    #[test]
    fn test_with_phrase_boost_builder() {
        let reranker = BM25Reranker::new().with_phrase_boost(0.7);
        assert_eq!(reranker.phrase_boost, 0.7);

        // Test clamping
        let clamped = BM25Reranker::new().with_phrase_boost(5.0);
        assert_eq!(clamped.phrase_boost, 2.0); // Clamped to max
    }

    #[test]
    fn test_phrase_bonus_calculation() {
        let reranker = BM25Reranker::new();

        // Query: "knowledge graph"
        // Doc: "...knowledge graph extraction..."
        let query = vec!["knowledge".to_string(), "graph".to_string()];
        let doc_with_phrase = vec![
            "some".to_string(),
            "knowledge".to_string(),
            "graph".to_string(),
            "extraction".to_string(),
        ];
        let doc_without_phrase = vec![
            "graph".to_string(),
            "of".to_string(),
            "knowledge".to_string(),
        ];

        let bonus_with = reranker.compute_phrase_bonus(&query, &doc_with_phrase);
        let bonus_without = reranker.compute_phrase_bonus(&query, &doc_without_phrase);

        assert!(
            bonus_with > 0.0,
            "Should have phrase bonus for adjacent terms"
        );
        assert_eq!(
            bonus_without, 0.0,
            "Should have no bonus for non-adjacent terms"
        );
    }

    #[tokio::test]
    async fn test_phrase_boost_ranking_effect() {
        // Phrase boost should prefer "knowledge graph" over "graph knowledge"
        let no_boost = BM25Reranker::new(); // phrase_boost = 0
        let with_boost = BM25Reranker::for_semantic(); // phrase_boost = 0.5

        let query = "knowledge graph";
        let documents = vec![
            "This document discusses knowledge graph extraction.".to_string(),
            "The graph of knowledge is complex.".to_string(),
            "Something about graphs and some knowledge.".to_string(),
        ];

        let results_no_boost = no_boost.rerank(query, &documents, None).await.unwrap();
        let results_with_boost = with_boost.rerank(query, &documents, None).await.unwrap();

        // Both should have doc 0 or 1 in top position (both contain the terms)
        // With phrase boost, doc 0 should score higher
        let phrase_doc_score_boosted = results_with_boost
            .iter()
            .find(|r| r.index == 0)
            .unwrap()
            .relevance_score;
        let non_phrase_doc_score_boosted = results_with_boost
            .iter()
            .find(|r| r.index == 1)
            .unwrap()
            .relevance_score;

        assert!(
            phrase_doc_score_boosted > non_phrase_doc_score_boosted,
            "Phrase match should score higher with boost: {} vs {}",
            phrase_doc_score_boosted,
            non_phrase_doc_score_boosted
        );

        // Verify boost has effect
        let phrase_score_no_boost = results_no_boost
            .iter()
            .find(|r| r.index == 0)
            .unwrap()
            .relevance_score;
        assert!(
            phrase_doc_score_boosted > phrase_score_no_boost,
            "Boosted score should be higher than non-boosted"
        );
    }

    #[test]
    fn test_phrase_bonus_edge_cases() {
        let reranker = BM25Reranker::new();

        // Empty query
        let bonus_empty = reranker.compute_phrase_bonus(&[], &["test".to_string()]);
        assert_eq!(bonus_empty, 0.0);

        // Single term query (no pairs)
        let bonus_single = reranker.compute_phrase_bonus(
            &["test".to_string()],
            &["test".to_string(), "doc".to_string()],
        );
        assert_eq!(bonus_single, 0.0);

        // Empty document
        let bonus_empty_doc =
            reranker.compute_phrase_bonus(&["a".to_string(), "b".to_string()], &[]);
        assert_eq!(bonus_empty_doc, 0.0);
    }

    #[tokio::test]
    async fn test_short_docs_preset_behavior() {
        // Short docs preset should handle short content well
        let short_reranker = BM25Reranker::for_short_docs();
        let long_reranker = BM25Reranker::for_long_docs();

        let query = "rust programming";
        let short_docs = vec![
            "Rust programming language".to_string(),
            "Python is great".to_string(),
        ];

        let short_results = short_reranker
            .rerank(query, &short_docs, None)
            .await
            .unwrap();
        let long_results = long_reranker
            .rerank(query, &short_docs, None)
            .await
            .unwrap();

        // Both should rank correctly
        assert_eq!(short_results[0].index, 0);
        assert_eq!(long_results[0].index, 0);
    }

    #[tokio::test]
    async fn test_technical_preset_exact_matching() {
        // Technical preset should prefer exact matches (no stemming)
        let tech = BM25Reranker::for_technical();
        let rag = BM25Reranker::for_rag();

        let query = "running";
        let documents = vec![
            "The process is running".to_string(),
            "The runner runs quickly".to_string(), // Contains "runs" (stemmed = "run")
        ];

        let tech_results = tech.rerank(query, &documents, None).await.unwrap();
        let rag_results = rag.rerank(query, &documents, None).await.unwrap();

        // Technical should prefer exact "running" match
        assert_eq!(tech_results[0].index, 0);

        // RAG with stemming might score "runs" higher due to stem matching
        // Both documents are relevant, but exact match wins for technical
        assert!(tech_results[0].relevance_score > 0.0);
        assert!(rag_results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_plus_long_document_handling() {
        // BM25+ should help long documents score better
        let bm25 = BM25Reranker::new(); // Standard BM25 (delta = 0)
        let bm25_plus = BM25Reranker::bm25_plus(); // BM25+ (delta = 1)

        let query = "Peugeot";
        let documents = vec![
            "Peugeot".to_string(), // Short doc with term
            "Peugeot cars are great. They have been making cars for over a century. The French automaker is known for quality. They produce sedans, SUVs, and more.".to_string(), // Long doc with term
        ];

        let results_bm25 = bm25.rerank(query, &documents, None).await.unwrap();
        let results_bm25_plus = bm25_plus.rerank(query, &documents, None).await.unwrap();

        // Both should rank docs with the term, but BM25+ gives long doc higher relative score
        assert!(results_bm25[0].relevance_score > 0.0);
        assert!(results_bm25_plus[0].relevance_score > 0.0);

        // BM25+ should give higher score to long doc compared to standard BM25
        // Because delta adds a floor to the score contribution
        let bm25_long_score = results_bm25
            .iter()
            .find(|r| r.index == 1)
            .unwrap()
            .relevance_score;
        let bm25_plus_long_score = results_bm25_plus
            .iter()
            .find(|r| r.index == 1)
            .unwrap()
            .relevance_score;
        assert!(
            bm25_plus_long_score > bm25_long_score,
            "BM25+ should score long doc higher"
        );
    }

    #[test]
    fn test_bm25_params_clamping() {
        // k1 should be clamped to [0, 3]
        let reranker = BM25Reranker::with_full_params(10.0, 2.0, -1.0);
        assert_eq!(reranker.k1, 3.0); // Clamped from 10.0
        assert_eq!(reranker.b, 1.0); // Clamped from 2.0
        assert_eq!(reranker.delta, 0.0); // Clamped from -1.0
    }

    // =========== TermOverlapReranker Tests ===========

    #[tokio::test]
    async fn test_term_overlap_reranker() {
        let reranker = TermOverlapReranker::new();
        let query = "capital of France";
        let documents = vec![
            "The capital of France is Paris.".to_string(),
            "Tokyo is the capital of Japan.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // France doc should score higher (has "France" + "capital" + "of")
        assert_eq!(results[0].index, 0);
        assert_eq!(reranker.name(), "term-overlap");
    }

    #[test]
    fn test_mock_reranker_alias() {
        // Verify MockReranker is a type alias for TermOverlapReranker
        let mock: MockReranker = MockReranker::new();
        let term_overlap: TermOverlapReranker = TermOverlapReranker::new();

        // Both should have same behavior
        assert_eq!(mock.name(), term_overlap.name());
    }

    // =========== RRF Reranker Tests ===========

    #[test]
    fn test_rrf_fusion_basic() {
        let rrf = RRFReranker::new();

        // Two ranking lists
        let list1 = vec![0, 1, 2]; // BM25 ranking: doc0 best, then doc1, doc2
        let list2 = vec![2, 1, 0]; // Vector ranking: doc2 best, then doc1, doc0

        let results = rrf.fuse(&[list1, list2], 3);

        // Doc1 should be top because it's rank 2 in both lists
        // score(doc1) = 1/(60+2) + 1/(60+2) = 0.032
        // score(doc0) = 1/(60+1) + 1/(60+3) = 0.016 + 0.016 = 0.032
        // score(doc2) = 1/(60+3) + 1/(60+1) = same as doc0
        // All should have similar scores (middle ranks)
        assert!(!results.is_empty());
    }

    #[test]
    fn test_rrf_fusion_clear_winner() {
        let rrf = RRFReranker::with_k(1); // Low k for clearer differences

        // Doc 0 is first in both lists
        let list1 = vec![0, 1, 2];
        let list2 = vec![0, 2, 1];

        let results = rrf.fuse(&[list1, list2], 3);

        // Doc 0 should be first (rank 1 in both)
        assert_eq!(results[0].index, 0);
        // Score should be 1/(1+1) + 1/(1+1) = 1.0
        assert!((results[0].relevance_score - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_rrf_reranker_trait() {
        let reranker = RRFReranker::new();
        let query = "test query";
        let documents = vec![
            "First document about test.".to_string(),
            "Second document.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        assert_eq!(results.len(), 2);
        // First doc should score higher (has "test")
        assert_eq!(results[0].index, 0);
    }

    // =========== Hybrid Reranker Tests ===========

    #[tokio::test]
    async fn test_hybrid_reranker_without_vector() {
        let reranker = HybridReranker::new();
        let query = "test query";
        let documents = vec![
            "This is a test document.".to_string(),
            "Another document here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should work like BM25 without vector rankings
        assert_eq!(results[0].index, 0); // "test" appears in doc 0
    }

    #[tokio::test]
    async fn test_hybrid_reranker_with_vector() {
        let reranker = HybridReranker::new();
        let query = "test query";
        let documents = vec![
            "This is a test document.".to_string(),
            "Another document here.".to_string(),
            "Third one with test.".to_string(),
        ];

        // Vector search says doc 1 is best (different from BM25)
        let vector_rankings = vec![1, 2, 0];

        let results = reranker
            .rerank_hybrid(query, &documents, Some(vector_rankings), None)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        // Result should balance BM25 (prefers 0, 2) with vector (prefers 1)
    }

    #[test]
    fn test_hybrid_reranker_defaults() {
        let reranker = HybridReranker::new();
        assert_eq!(reranker.name(), "hybrid");
        assert_eq!(reranker.model(), "hybrid-reranker");
    }

    // =========== Edge Case Tests (OODA Loop 13-14) ===========

    #[tokio::test]
    async fn test_bm25_very_long_document() {
        let reranker = BM25Reranker::new();
        let query = "Peugeot 2008";

        // Create a very long document with the target terms buried inside
        let long_prefix = "Lorem ipsum dolor sit amet. ".repeat(100);
        let long_doc = format!(
            "{}The Peugeot 2008 is an excellent SUV.{}",
            long_prefix, long_prefix
        );

        let documents = vec![
            "Peugeot 2008 is great.".to_string(), // Short focused doc
            long_doc.clone(),                     // Long doc with same info
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Short focused doc should score better due to length normalization
        assert_eq!(results[0].index, 0, "Short focused doc should rank first");
    }

    #[tokio::test]
    async fn test_bm25_special_characters() {
        let reranker = BM25Reranker::new();
        let query = "C++ programming";
        let documents = vec![
            "C++ programming is powerful.".to_string(),
            "Python programming is easy.".to_string(),
            "Programming in general is fun.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should still work with special characters filtered out
        assert!(!results.is_empty());
        // Doc with "programming" should score
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_stop_words() {
        let reranker = BM25Reranker::new();
        // Query with common words that might appear in all docs
        let query = "the a an is are";
        let documents = vec![
            "The cat is on the mat.".to_string(),
            "A dog is in the yard.".to_string(),
            "Completely different content here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // All docs should have similar low scores since query is mostly stop words
        // (Note: We filter single-char tokens, so scores should be low overall)
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_bm25_multiple_exact_matches() {
        let reranker = BM25Reranker::new();
        let query = "Peugeot 2008";
        let documents = vec![
            "Peugeot 2008".to_string(),               // Exact match
            "Peugeot 2008 Peugeot 2008".to_string(),  // Double match
            "The Peugeot 2008 is a car.".to_string(), // Match with context
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Exact match should score very high
        // Double match has more TF but longer doc, so saturation should help
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_case_insensitivity() {
        let reranker = BM25Reranker::new();
        let query = "PEUGEOT ENVY";
        let documents = vec![
            "peugeot envy model".to_string(),
            "Peugeot Envy SUV".to_string(),
            "PEUGEOT ENVY 2024".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // All should match equally (case insensitive)
        // All docs have both terms, so scores should be similar
        let score_variance = results
            .iter()
            .map(|r| r.relevance_score)
            .fold((0.0, 0.0), |acc, s| {
                (acc.0 + s, (acc.1 - s).abs().max(acc.1))
            });
        assert!(score_variance.0 > 0.0, "Should have non-zero scores");
    }

    #[tokio::test]
    async fn test_bm25_numeric_precision() {
        let reranker = BM25Reranker::new();
        // Critical: test that different numbers are distinguished
        let query = "model 2008";
        let documents = vec![
            "Model 2007 released last year.".to_string(),
            "Model 2008 released this year.".to_string(),
            "Model 2009 coming next year.".to_string(),
            "Model 208 is different.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // "2008" should match exactly only doc 1
        assert_eq!(results[0].index, 1, "Exact year match should rank first");
    }

    #[tokio::test]
    async fn test_bm25_unicode_comprehensive() {
        let reranker = BM25Reranker::new();
        let query = "cafe francais elegance";
        let documents = vec![
            "Le café français a de l'élégance.".to_string(),
            "The French cafe has elegance.".to_string(), // English version
            "Random unrelated text here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Both French and English docs should score well
        assert!(results[0].relevance_score > results[2].relevance_score);
        assert!(results[1].relevance_score > results[2].relevance_score);
    }

    #[tokio::test]
    async fn test_bm25_single_document() {
        let reranker = BM25Reranker::new();
        let query = "test query";
        let documents = vec!["Single document with test.".to_string()];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);
        // With single doc, IDF is 0 (ln(1/1 + 1) = ln(1.5) ≈ 0.4)
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_vs_mock_comparison() {
        // Comparative test: BM25 should outperform MockReranker on precision
        let bm25 = BM25Reranker::new();
        let mock = MockReranker::new();

        let query = "2008";
        let documents = vec![
            "The Peugeot 208 is a compact car.".to_string(),
            "The Peugeot 2008 is an SUV.".to_string(),
        ];

        let bm25_results = bm25.rerank(query, &documents, None).await.unwrap();
        let _mock_results = mock.rerank(query, &documents, None).await.unwrap();

        // BM25 should correctly identify "2008" (index 1) as top
        assert_eq!(bm25_results[0].index, 1, "BM25 should rank 2008 first");

        // MockReranker might fail this (term overlap doesn't distinguish)
        // This test documents the improvement
    }

    #[tokio::test]
    async fn test_rrf_empty_rankings() {
        let reranker = RRFReranker::new();
        let results = reranker.fuse(&[], 5);
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_rrf_single_ranking() {
        let reranker = RRFReranker::new();
        let single_list = vec![2, 0, 1]; // doc2 first, then doc0, doc1
        let results = reranker.fuse(&[single_list], 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].index, 2);
    }

    // =========== Stress Tests (OODA Loop 16) ===========

    #[tokio::test]
    async fn test_bm25_stress_100_documents() {
        let reranker = BM25Reranker::new();
        let query = "target keyword";

        // Create 100 documents, only one contains the target
        let mut documents: Vec<String> = (0..99)
            .map(|i| {
                format!(
                    "Document {} has some random content about various topics.",
                    i
                )
            })
            .collect();
        documents
            .push("This document contains the target keyword we are searching for.".to_string());

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Document with target should be first
        assert_eq!(results[0].index, 99, "Target document should rank first");
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_stress_1000_documents() {
        let reranker = BM25Reranker::new();
        let query = "unique identifier xyz";

        // Create 1000 documents
        let mut documents: Vec<String> = (0..999)
            .map(|i| format!("Document number {} with generic content.", i))
            .collect();
        documents.push("This text contains unique identifier xyz marker.".to_string());

        let start = std::time::Instant::now();
        let results = reranker.rerank(query, &documents, Some(10)).await.unwrap();
        let elapsed = start.elapsed();

        // Performance: should complete in under 1 second
        assert!(
            elapsed.as_millis() < 1000,
            "Should complete in under 1s, took {:?}",
            elapsed
        );

        // Precision: target should be in top 10
        assert!(
            results.iter().any(|r| r.index == 999),
            "Target should be in top 10"
        );
    }

    #[tokio::test]
    async fn test_bm25_stress_long_query() {
        let reranker = BM25Reranker::new();
        // Very long query with many terms
        let query = "The quick brown fox jumps over the lazy dog and runs through the forest while the sun sets behind the mountains creating beautiful shadows";

        let documents = vec![
            "A fox runs quickly.".to_string(),
            "The brown dog is lazy.".to_string(),
            "Mountains have beautiful sunsets.".to_string(),
            "Completely unrelated content here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // All docs with matching terms should score
        assert!(results[0].relevance_score > results[3].relevance_score);
    }

    #[tokio::test]
    async fn test_bm25_stress_repeated_terms() {
        let reranker = BM25Reranker::new();
        // Query with repeated terms
        let query = "test test test test test";

        let documents = vec![
            "This is a test document.".to_string(),
            "Another test here.".to_string(),
            "No relevant content.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should handle repeated terms gracefully (TF saturation)
        assert_eq!(results.len(), 3);
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_bm25_stress_all_same_content() {
        let reranker = BM25Reranker::new();
        let query = "test content";

        // All documents have identical content
        let documents: Vec<String> = (0..10)
            .map(|_| "This is test content for evaluation.".to_string())
            .collect();

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // All should have the same score
        let first_score = results[0].relevance_score;
        for result in &results {
            assert!((result.relevance_score - first_score).abs() < 0.001);
        }
    }

    #[tokio::test]
    async fn test_bm25_stress_empty_documents_in_list() {
        let reranker = BM25Reranker::new();
        let query = "test query";

        let documents = vec![
            "".to_string(),
            "   ".to_string(),
            "Actual content with test.".to_string(),
            "".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Non-empty document should rank first
        assert_eq!(results[0].index, 2);
    }

    #[tokio::test]
    async fn test_bm25_stress_unicode_heavy() {
        let reranker = BM25Reranker::new();
        let query = "recherche voiture";

        let documents = vec![
            "Recherche de voiture électrique en France.".to_string(),
            "日本語テスト文書".to_string(), // Japanese
            "مستند عربي".to_string(),       // Arabic
            "Русский документ".to_string(), // Russian
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // French doc should rank first
        assert_eq!(results[0].index, 0);
    }

    // =========== Boundary Condition Tests (OODA Loop 17-18) ===========

    #[tokio::test]
    async fn test_bm25_boundary_top_n_zero() {
        let reranker = BM25Reranker::new();
        let query = "test";
        let documents = vec!["test document".to_string()];

        let results = reranker.rerank(query, &documents, Some(0)).await.unwrap();

        // top_n = 0 should return empty
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_bm25_boundary_top_n_larger_than_docs() {
        let reranker = BM25Reranker::new();
        let query = "test";
        let documents = vec!["test one".to_string(), "test two".to_string()];

        let results = reranker.rerank(query, &documents, Some(100)).await.unwrap();

        // Should return all 2 docs, not 100
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_bm25_boundary_single_char_query() {
        let reranker = BM25Reranker::new();
        let query = "a";
        let documents = vec!["a document".to_string(), "another".to_string()];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Single char filtered out, should have 0 scores
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_bm25_boundary_whitespace_only_query() {
        let reranker = BM25Reranker::new();
        let query = "   \t\n   ";
        let documents = vec!["some document".to_string()];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Whitespace query should return all docs with 0 score
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relevance_score, 0.0);
    }

    #[tokio::test]
    async fn test_bm25_boundary_very_short_doc() {
        let reranker = BM25Reranker::new();
        let query = "test";
        let documents = vec![
            "t".to_string(),    // Too short
            "test".to_string(), // Exact match
            "testing".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Exact match should rank high
        assert!(results[0].index == 1 || results[1].index == 1);
    }

    #[tokio::test]
    async fn test_bm25_french_peugeot_full_spec() {
        let reranker = BM25Reranker::new();
        let query = "Peugeot 2008 ENVY motorisation";

        // Realistic car spec documents
        let documents = vec![
            "Peugeot 2008 ENVY: Motorisation PureTech 130ch, boîte automatique EAT8.".to_string(),
            "Peugeot 208 GT: Moteur BlueHDi 100ch, boîte manuelle 6 vitesses.".to_string(),
            "Citroën C3 Aircross: Essence PureTech 110ch.".to_string(),
            "Renault Captur: Moteur TCe 100ch hybride.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Peugeot 2008 ENVY doc should be first (has all terms)
        assert_eq!(results[0].index, 0, "2008 ENVY doc should rank first");

        // Should have significantly higher score
        assert!(results[0].relevance_score > results[1].relevance_score * 1.2);
    }

    #[tokio::test]
    async fn test_rrf_boundary_many_rankings() {
        let reranker = RRFReranker::new();

        // 10 ranking lists all agreeing
        let rankings: Vec<Vec<usize>> = (0..10).map(|_| vec![0, 1, 2]).collect();

        let results = reranker.fuse(&rankings, 3);

        // Doc 0 should have highest score (first in all 10 lists)
        assert_eq!(results[0].index, 0);
    }

    #[tokio::test]
    async fn test_hybrid_boundary_no_vector_rankings() {
        let reranker = HybridReranker::new();
        let query = "test query";
        let documents = vec![
            "test document here".to_string(),
            "another document".to_string(),
        ];

        // No vector rankings provided
        let results = reranker
            .rerank_hybrid(query, &documents, None, None)
            .await
            .unwrap();

        // Should fall back to BM25 only
        assert_eq!(results.len(), 2);
        assert!(results[0].relevance_score >= results[1].relevance_score);
    }

    // =========================================================================
    // Enhanced Tokenizer Tests (OODA Loop 2)
    // =========================================================================

    #[test]
    fn test_tokenizer_config_default() {
        let config = TokenizerConfig::default();
        assert!(config.enable_stemming);
        assert!(config.enable_stop_words);
        assert_eq!(config.min_token_length, 2);
    }

    #[test]
    fn test_tokenizer_config_minimal() {
        let config = TokenizerConfig::minimal();
        assert!(!config.enable_stemming);
        assert!(!config.enable_stop_words);
    }

    #[test]
    fn test_tokenizer_config_enhanced() {
        let config = TokenizerConfig::enhanced();
        assert!(config.enable_stemming);
        assert!(config.enable_stop_words);
    }

    #[test]
    fn test_enhanced_tokenizer_unicode_normalization() {
        let reranker = BM25Reranker::new_enhanced();
        // Test various Unicode accents are normalized
        let tokens = reranker.tokenize_with_config("café résumé naïve");
        assert!(tokens.contains(&"cafe".to_string()) || tokens.contains(&"caf".to_string()));
        assert!(tokens.contains(&"resum".to_string()) || tokens.contains(&"resume".to_string()));
        assert!(tokens.contains(&"naiv".to_string()) || tokens.contains(&"naive".to_string()));
    }

    #[test]
    fn test_enhanced_tokenizer_stemming() {
        let reranker = BM25Reranker::new_enhanced();
        // Test stemming of English words
        let tokens = reranker.tokenize_with_config("running jumps played");
        // Porter2 stems: running → run, jumps → jump, played → play
        assert!(tokens.contains(&"run".to_string()));
        assert!(tokens.contains(&"jump".to_string()));
        assert!(tokens.contains(&"play".to_string()));
    }

    #[test]
    fn test_enhanced_tokenizer_stop_words() {
        let reranker = BM25Reranker::new_enhanced();
        let tokens = reranker.tokenize_with_config("the quick brown fox");
        // "the" should be filtered as a stop word
        assert!(!tokens.iter().any(|t| t == "the"));
        // "quick", "brown", "fox" should remain (possibly stemmed)
        assert!(tokens.len() >= 2);
    }

    #[test]
    fn test_enhanced_tokenizer_preserves_meaning() {
        let reranker = BM25Reranker::new_enhanced();
        // Test that meaningful terms are preserved after processing
        let tokens = reranker.tokenize_with_config("artificial intelligence machine learning");
        // Should contain stems of key terms
        assert!(tokens
            .iter()
            .any(|t| t.contains("artifici") || t.contains("artificial")));
        assert!(tokens
            .iter()
            .any(|t| t.contains("intellig") || t.contains("intelligence")));
        assert!(tokens
            .iter()
            .any(|t| t.contains("machin") || t.contains("machine")));
        assert!(tokens
            .iter()
            .any(|t| t.contains("learn") || t.contains("learning")));
    }

    #[test]
    fn test_minimal_tokenizer_no_stemming() {
        let _reranker = BM25Reranker::new();
        // Minimal tokenizer should NOT stem
        let tokens = BM25Reranker::tokenize("running jumps played");
        assert!(tokens.contains(&"running".to_string()));
        assert!(tokens.contains(&"jumps".to_string()));
        assert!(tokens.contains(&"played".to_string()));
    }

    #[test]
    fn test_bm25_with_tokenizer_config() {
        let reranker = BM25Reranker::new().with_tokenizer_config(TokenizerConfig::enhanced());
        // Verify the config was applied
        assert!(reranker.tokenizer_config.enable_stemming);
    }

    #[tokio::test]
    async fn test_enhanced_bm25_improves_recall() {
        // This test verifies that stemming improves recall by matching morphological variants
        let reranker = BM25Reranker::new_enhanced();
        let documents = vec![
            "The runner runs daily".to_string(), // contains "runner", "runs"
            "Swimming is good exercise".to_string(), // unrelated
            "He was running yesterday".to_string(), // contains "running"
        ];

        let results = reranker.rerank("run", &documents, Some(3)).await.unwrap();

        // With stemming, "run" should match "runner", "runs", "running"
        // Documents 0 and 2 should rank higher than document 1
        let top_indices: Vec<usize> = results.iter().take(2).map(|r| r.index).collect();
        assert!(top_indices.contains(&0) || top_indices.contains(&2));
    }

    #[test]
    fn test_french_tokenizer() {
        let reranker = BM25Reranker::new().with_tokenizer_config(TokenizerConfig::french());
        let tokens = reranker.tokenize_with_config("parlons français facilement");
        // French stemmer should stem these
        assert!(!tokens.is_empty());
    }

    // =========================================================================
    // IDF Optimization Tests (OODA Loop 4)
    // =========================================================================

    #[test]
    fn test_document_frequency_computation() {
        let docs = vec![
            vec!["the".to_string(), "quick".to_string(), "brown".to_string()],
            vec!["the".to_string(), "lazy".to_string(), "dog".to_string()],
            vec!["quick".to_string(), "fox".to_string()],
        ];

        let df_map = BM25Reranker::compute_document_frequencies(&docs);

        assert_eq!(df_map.get("the"), Some(&2)); // appears in 2 docs
        assert_eq!(df_map.get("quick"), Some(&2)); // appears in 2 docs
        assert_eq!(df_map.get("brown"), Some(&1)); // appears in 1 doc
        assert_eq!(df_map.get("fox"), Some(&1)); // appears in 1 doc
        assert_eq!(df_map.get("missing"), None); // doesn't exist
    }

    #[test]
    fn test_idf_from_df_equivalence() {
        // Verify compute_idf_from_df produces same result as compute_idf
        let docs = vec![
            vec!["apple".to_string(), "banana".to_string()],
            vec!["apple".to_string(), "cherry".to_string()],
            vec!["banana".to_string(), "date".to_string()],
        ];

        let n = docs.len() as f64;
        let df_map = BM25Reranker::compute_document_frequencies(&docs);

        // Test "apple" (appears in 2/3 docs)
        let idf_old = BM25Reranker::compute_idf("apple", &docs);
        let idf_new = BM25Reranker::compute_idf_from_df(n, *df_map.get("apple").unwrap() as f64);
        assert!((idf_old - idf_new).abs() < 1e-10);

        // Test "date" (appears in 1/3 docs)
        let idf_old = BM25Reranker::compute_idf("date", &docs);
        let idf_new = BM25Reranker::compute_idf_from_df(n, *df_map.get("date").unwrap() as f64);
        assert!((idf_old - idf_new).abs() < 1e-10);
    }

    #[test]
    fn test_repeated_terms_in_document() {
        // Ensure repeated terms in a single doc count as 1 for DF
        let docs = vec![
            vec!["the".to_string(), "the".to_string(), "the".to_string()], // 3x "the"
            vec!["cat".to_string()],
        ];

        let df_map = BM25Reranker::compute_document_frequencies(&docs);

        // "the" appears 3 times but only in 1 document
        assert_eq!(df_map.get("the"), Some(&1));
        assert_eq!(df_map.get("cat"), Some(&1));
    }

    #[test]
    fn test_idf_edge_cases() {
        // Test IDF for term in all documents (should be low but positive)
        let idf_all = BM25Reranker::compute_idf_from_df(10.0, 10.0);
        assert!(idf_all > 0.0); // +1 in formula ensures non-negative

        // Test IDF for term in no documents (should be high)
        let idf_none = BM25Reranker::compute_idf_from_df(10.0, 0.0);
        assert!(idf_none > idf_all);

        // Test IDF for term in half the documents
        let idf_half = BM25Reranker::compute_idf_from_df(10.0, 5.0);
        assert!(idf_half > idf_all);
        assert!(idf_half < idf_none);
    }

    // =========== Performance Benchmarks (OODA Loop 6) ===========

    #[tokio::test]
    async fn test_performance_minimal_vs_enhanced_1000_docs() {
        // Compare performance: minimal tokenization vs enhanced
        let minimal = BM25Reranker::new(); // No stemming
        let enhanced = BM25Reranker::new_enhanced(); // With stemming

        let query = "running quickly through the forest";
        let documents: Vec<String> = (0..1000)
            .map(|i| {
                format!(
                    "Document {} with content about running and forests and trees.",
                    i
                )
            })
            .collect();

        // Warm up
        let _ = minimal.rerank(query, &documents, Some(10)).await;
        let _ = enhanced.rerank(query, &documents, Some(10)).await;

        // Benchmark minimal
        let start = std::time::Instant::now();
        for _ in 0..5 {
            let _ = minimal.rerank(query, &documents, Some(10)).await;
        }
        let minimal_time = start.elapsed() / 5;

        // Benchmark enhanced
        let start = std::time::Instant::now();
        for _ in 0..5 {
            let _ = enhanced.rerank(query, &documents, Some(10)).await;
        }
        let enhanced_time = start.elapsed() / 5;

        // Enhanced should be no more than 3x slower (stemming overhead)
        let ratio = enhanced_time.as_micros() as f64 / minimal_time.as_micros().max(1) as f64;
        assert!(
            ratio < 3.0,
            "Enhanced tokenization should be at most 3x slower, was {:.2}x",
            ratio
        );

        // Both should complete in reasonable time
        assert!(
            minimal_time.as_millis() < 500,
            "Minimal should complete in <500ms"
        );
        assert!(
            enhanced_time.as_millis() < 1000,
            "Enhanced should complete in <1s"
        );
    }

    #[tokio::test]
    async fn test_performance_scale_comparison() {
        // Test how performance scales with document count
        let reranker = BM25Reranker::for_rag();
        let query = "knowledge graph entity extraction";

        let mut times: Vec<(usize, std::time::Duration)> = Vec::new();

        for count in [100, 500, 1000, 2000] {
            let documents: Vec<String> = (0..count)
                .map(|i| {
                    format!(
                        "Document {} discusses knowledge graphs and entity extraction methods.",
                        i
                    )
                })
                .collect();

            let start = std::time::Instant::now();
            let _ = reranker.rerank(query, &documents, Some(10)).await;
            let elapsed = start.elapsed();
            times.push((count, elapsed));
        }

        // Linear scaling check: 2000 docs should be at most 4x slower than 500 docs
        let time_500 = times.iter().find(|(c, _)| *c == 500).unwrap().1;
        let time_2000 = times.iter().find(|(c, _)| *c == 2000).unwrap().1;

        let scale_factor = time_2000.as_micros() as f64 / time_500.as_micros().max(1) as f64;
        assert!(
            scale_factor < 6.0, // Allow some overhead
            "Scaling should be near-linear, was {:.2}x for 4x documents",
            scale_factor
        );
    }

    #[tokio::test]
    async fn test_performance_presets_comparison() {
        // Verify all presets complete in reasonable time
        let presets: Vec<(&str, BM25Reranker)> = vec![
            ("minimal", BM25Reranker::new()),
            ("enhanced", BM25Reranker::new_enhanced()),
            ("short_docs", BM25Reranker::for_short_docs()),
            ("long_docs", BM25Reranker::for_long_docs()),
            ("technical", BM25Reranker::for_technical()),
            ("rag", BM25Reranker::for_rag()),
        ];

        let query = "machine learning neural networks deep learning";
        let documents: Vec<String> = (0..500)
            .map(|i| format!("Research paper {} on machine learning and AI systems.", i))
            .collect();

        for (name, reranker) in presets {
            let start = std::time::Instant::now();
            let results = reranker.rerank(query, &documents, Some(10)).await.unwrap();
            let elapsed = start.elapsed();

            assert!(
                elapsed.as_millis() < 500,
                "Preset '{}' should complete in <500ms, took {:?}",
                name,
                elapsed
            );
            assert_eq!(results.len(), 10, "Should return 10 results");
        }
    }

    // =========== Edge Case Tests (OODA Loop 8) ===========

    #[tokio::test]
    async fn test_edge_case_stop_words_only_query() {
        // Query with only stop words should still work
        let reranker = BM25Reranker::new_enhanced();
        let query = "the and or but";

        let documents = vec![
            "The quick brown fox.".to_string(),
            "Something completely different.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should return results without panicking
        assert_eq!(results.len(), 2);
        // All stop words filtered = zero score for all
        assert!(results.iter().all(|r| r.relevance_score >= 0.0));
    }

    #[tokio::test]
    async fn test_edge_case_numeric_only_query() {
        // Numeric queries like years should work
        let reranker = BM25Reranker::new();
        let query = "2024";

        let documents = vec![
            "Event in 2024 was successful.".to_string(),
            "Looking back at 2023.".to_string(),
            "No year mentioned here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // 2024 doc should rank first
        assert_eq!(results[0].index, 0, "2024 document should rank first");
        assert!(results[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_edge_case_no_matching_terms() {
        // When no terms match, all scores should be 0
        let reranker = BM25Reranker::new();
        let query = "xyzzy qwerty asdfgh";

        let documents = vec![
            "The quick brown fox.".to_string(),
            "Lazy dog sleeps.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // All scores should be 0
        assert!(
            results.iter().all(|r| r.relevance_score == 0.0),
            "No matching terms should give zero scores"
        );
    }

    #[tokio::test]
    async fn test_edge_case_identical_documents() {
        // Identical documents should have identical scores
        let reranker = BM25Reranker::new();
        let query = "test query";

        let documents = vec![
            "This is a test document with test query terms.".to_string(),
            "This is a test document with test query terms.".to_string(),
            "Different document content here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // First two docs are identical, should have same score
        let score0 = results
            .iter()
            .find(|r| r.index == 0)
            .unwrap()
            .relevance_score;
        let score1 = results
            .iter()
            .find(|r| r.index == 1)
            .unwrap()
            .relevance_score;
        assert!(
            (score0 - score1).abs() < 0.0001,
            "Identical docs should have identical scores"
        );
    }

    #[tokio::test]
    async fn test_edge_case_very_long_repeated_term() {
        // Document with term repeated many times shouldn't cause overflow
        let reranker = BM25Reranker::new();
        let query = "test";

        // 1000 repetitions of "test"
        let repeated_doc = std::iter::repeat("test")
            .take(1000)
            .collect::<Vec<_>>()
            .join(" ");
        let documents = vec![repeated_doc, "test document".to_string()];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should complete without overflow
        assert!(results[0].relevance_score.is_finite());
        assert!(results[1].relevance_score.is_finite());
    }

    #[tokio::test]
    async fn test_edge_case_mixed_case_query() {
        // Mixed case shouldn't affect results
        let reranker = BM25Reranker::new();

        let documents = vec![
            "Rust programming language.".to_string(),
            "Python is popular.".to_string(),
        ];

        let results_lower = reranker.rerank("rust", &documents, None).await.unwrap();
        let results_upper = reranker.rerank("RUST", &documents, None).await.unwrap();
        let results_mixed = reranker.rerank("RuSt", &documents, None).await.unwrap();

        // All should rank the same
        assert_eq!(results_lower[0].index, results_upper[0].index);
        assert_eq!(results_lower[0].index, results_mixed[0].index);
    }

    #[tokio::test]
    async fn test_edge_case_punctuation_in_query() {
        // Punctuation should be handled correctly
        let reranker = BM25Reranker::new();
        let query = "Hello, World!";

        let documents = vec![
            "Hello World program.".to_string(),
            "Goodbye World.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Hello World doc should rank first
        assert_eq!(results[0].index, 0);
    }

    #[test]
    fn test_edge_case_idf_extreme_values() {
        // IDF should handle extreme cases gracefully

        // Term in every document (very common)
        let idf_common = BM25Reranker::compute_idf_from_df(1000.0, 1000.0);
        assert!(idf_common.is_finite());
        assert!(idf_common >= 0.0); // Should be non-negative

        // Term in no documents (unknown term)
        let idf_rare = BM25Reranker::compute_idf_from_df(1000.0, 0.0);
        assert!(idf_rare.is_finite());
        assert!(idf_rare > idf_common); // Rare terms have higher IDF

        // Zero documents edge case
        let idf_zero = BM25Reranker::compute_idf_from_df(0.0, 0.0);
        assert!(idf_zero.is_finite());
    }

    // =========== Unicode Edge Case Tests (OODA Loop 9) ===========

    #[tokio::test]
    async fn test_unicode_cjk_chinese() {
        // Chinese characters (no spaces between words)
        let reranker = BM25Reranker::new();
        let query = "机器学习"; // Machine learning

        let documents = vec![
            "机器学习是人工智能的一个分支。".to_string(), // Contains query
            "深度学习很有趣。".to_string(),               // Different topic
            "Something in English.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should complete without error
        assert_eq!(results.len(), 3);
        // CJK chars are kept as-is (tokenization treats each char as token)
        assert!(results[0].relevance_score >= 0.0);
    }

    #[tokio::test]
    async fn test_unicode_emoji_in_content() {
        // Emoji should be handled gracefully
        let reranker = BM25Reranker::new();
        let query = "happy celebration";

        let documents = vec![
            "Happy celebration 🎉🎊🥳!".to_string(),
            "Sad day today.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Happy document should rank first despite emoji
        assert_eq!(results[0].index, 0);
        assert!(results[0].relevance_score > results[1].relevance_score);
    }

    #[tokio::test]
    async fn test_unicode_arabic_rtl() {
        // Arabic (right-to-left) text
        let reranker = BM25Reranker::new();
        let query = "مرحبا"; // Hello in Arabic

        let documents = vec![
            "مرحبا بكم في موقعنا.".to_string(), // Contains query
            "English text here.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Should handle RTL correctly
        assert_eq!(results.len(), 2);
        // Arabic doc should score higher if terms match
    }

    #[tokio::test]
    async fn test_unicode_math_symbols() {
        // Mathematical symbols in technical content
        let reranker = BM25Reranker::new();
        let query = "summation formula";

        let documents = vec![
            "The summation formula: ∑(x) = Σxᵢ".to_string(),
            "Simple math: 2 + 2 = 4".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Summation doc should rank first
        assert_eq!(results[0].index, 0);
    }

    #[tokio::test]
    async fn test_unicode_mixed_scripts() {
        // Document with multiple scripts
        let reranker = BM25Reranker::new();
        let query = "coffee";

        let documents = vec![
            "I love coffee ☕ and café au lait.".to_string(),
            "Tea is also good: 茶 お茶.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Coffee doc should rank first
        assert_eq!(results[0].index, 0);
    }

    #[tokio::test]
    async fn test_unicode_zero_width_characters() {
        // Zero-width joiners/spaces shouldn't break tokenization
        let reranker = BM25Reranker::new();
        let query = "test";

        let documents = vec![
            "This is a test\u{200B}document.".to_string(), // Zero-width space
            "Another test document.".to_string(),
        ];

        let results = reranker.rerank(query, &documents, None).await.unwrap();

        // Both should match "test"
        assert!(results[0].relevance_score > 0.0);
        assert!(results[1].relevance_score > 0.0);
    }
}
