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
            base_url: "https://dashscope.aliyuncs.com/api/v1/services/rerank/text-rerank/text-rerank".to_string(),
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

    fn build_request(&self, query: &str, documents: &[String], top_n: Option<usize>) -> serde_json::Value {
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
            ResponseFormat::Standard => {
                response.get("results")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default()
            }
            ResponseFormat::Aliyun => {
                response.get("output")
                    .and_then(|o| o.get("results"))
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default()
            }
        };

        if results.is_empty() {
            warn!("Rerank API returned empty results");
            return Ok(vec![]);
        }

        let mut rerank_results = Vec::with_capacity(results.len());
        for result in results {
            let index = result.get("index")
                .and_then(|i| i.as_u64())
                .ok_or_else(|| LlmError::Unknown("Missing index in rerank result".to_string()))?
                as usize;
            let score = result.get("relevance_score")
                .and_then(|s| s.as_f64())
                .ok_or_else(|| LlmError::Unknown("Missing relevance_score in rerank result".to_string()))?;
            
            rerank_results.push(RerankResult {
                index,
                relevance_score: score,
            });
        }

        // Sort by relevance score descending
        rerank_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

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

        debug!("Chunked {} documents into {} chunks", documents.len(), chunked.len());
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
                    ScoreAggregation::Max => scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    ScoreAggregation::Mean => scores.iter().sum::<f64>() / scores.len() as f64,
                    ScoreAggregation::First => scores[0],
                };
                RerankResult {
                    index: idx,
                    relevance_score: final_score,
                }
            })
            .collect();

        aggregated.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
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
        let api_top_n = if self.config.enable_chunking { None } else { top_n };

        let payload = self.build_request(query, &chunked_docs, api_top_n);

        debug!("Rerank request: {} documents, model: {}", chunked_docs.len(), self.config.model);

        let mut request = self.client
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

        let response_json: serde_json::Value = response.json().await
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

/// A mock reranker for testing that doesn't require API access.
pub struct MockReranker {
    model: String,
}

impl MockReranker {
    /// Create a new mock reranker.
    pub fn new() -> Self {
        Self {
            model: "mock-reranker".to_string(),
        }
    }
}

impl Default for MockReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for MockReranker {
    fn name(&self) -> &str {
        "mock"
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
        // Simple mock: score based on query term overlap
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
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        // Apply top_n
        if let Some(n) = top_n {
            results.truncate(n);
        }

        Ok(results)
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
            RerankResult { index: 0, relevance_score: 0.9 },
            RerankResult { index: 1, relevance_score: 0.8 },
            RerankResult { index: 2, relevance_score: 0.7 },
        ];
        let doc_indices = vec![0, 0, 1]; // Two chunks for doc 0, one for doc 1
        
        let aggregated = reranker.aggregate_scores(
            chunk_results,
            &doc_indices,
            2,
            ScoreAggregation::Max,
        );
        
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
}
