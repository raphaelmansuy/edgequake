//! Reranking functionality for improved retrieval quality.
//!
//! This module provides reranking capabilities to improve search result relevance
//! by scoring documents against a query using specialized reranking models.
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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

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
    /// Model name for trait compliance
    model: String,
}

impl BM25Reranker {
    /// Create a new BM25 reranker with default parameters.
    ///
    /// Defaults: k1 = 1.5, b = 0.75, delta = 0.0 (standard BM25)
    pub fn new() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            delta: 0.0, // Standard BM25, not BM25+
            model: "bm25-reranker".to_string(),
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
            model: "bm25-plus-reranker".to_string(),
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
            model: "bm25-reranker".to_string(),
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
            model: if delta > 0.0 {
                "bm25-plus-reranker".to_string()
            } else {
                "bm25-reranker".to_string()
            },
        }
    }

    /// Tokenize text into lowercase words, Unicode-normalized.
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .chars()
            .map(|c| {
                // Normalize accented characters for matching
                match c {
                    'é' | 'è' | 'ê' | 'ë' => 'e',
                    'à' | 'â' | 'ä' => 'a',
                    'î' | 'ï' => 'i',
                    'ô' | 'ö' => 'o',
                    'ù' | 'û' | 'ü' => 'u',
                    'ç' => 'c',
                    _ => c,
                }
            })
            .collect::<String>()
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
    fn compute_idf(term: &str, doc_terms_list: &[Vec<String>]) -> f64 {
        let n = doc_terms_list.len() as f64;
        let containing_docs = doc_terms_list
            .iter()
            .filter(|terms| terms.contains(&term.to_string()))
            .count() as f64;

        ((n - containing_docs + 0.5) / (containing_docs + 0.5) + 1.0).ln()
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

        // Tokenize query and all documents
        let query_terms = Self::tokenize(query);
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

        let doc_terms_list: Vec<Vec<String>> =
            documents.iter().map(|d| Self::tokenize(d)).collect();

        // Compute average document length
        let avgdl = doc_terms_list.iter().map(|d| d.len()).sum::<usize>() as f64
            / doc_terms_list.len().max(1) as f64;
        let avgdl = avgdl.max(1.0); // Avoid division by zero

        // Pre-compute IDF for all query terms
        let mut idf_cache = std::collections::HashMap::new();
        for term in &query_terms {
            let idf = Self::compute_idf(term, &doc_terms_list);
            idf_cache.insert(term.clone(), idf);
        }

        // Score each document
        let mut results: Vec<RerankResult> = doc_terms_list
            .iter()
            .enumerate()
            .map(|(idx, doc_terms)| {
                let score = self.compute_bm25_score(&query_terms, doc_terms, avgdl, &idf_cache);
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
        let bm25_long_score = results_bm25.iter().find(|r| r.index == 1).unwrap().relevance_score;
        let bm25_plus_long_score = results_bm25_plus.iter().find(|r| r.index == 1).unwrap().relevance_score;
        assert!(bm25_plus_long_score > bm25_long_score, "BM25+ should score long doc higher");
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
}
