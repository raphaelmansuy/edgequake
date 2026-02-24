//! RAG query engine for EdgeQuake.
//!
//! This module implements the core query engine that orchestrates retrieval-augmented
//! generation by combining vector similarity search, knowledge graph traversal,
//! and LLM-based answer synthesis.
//!
//! # Architecture
//!
//! The query engine follows a multi-stage pipeline:
//! 1. **Embedding**: Convert query text to vector representation
//! 2. **Retrieval**: Hybrid search across vector and graph storage
//! 3. **Reranking**: Score and filter retrieved chunks
//! 4. **Synthesis**: Generate final answer using LLM
//!
//! # Query Modes
//!
//! Each mode is implemented in its own sub-module:
//!
//! - `naive`: Simple vector similarity search
//! - `local`: Entity-centric retrieval with keyword extraction
//! - `global`: Relationship-centric high-level retrieval
//! - `mix`: Combines local + naive (recommended default)
//! - `hybrid`: Combines local + global with round-robin interleaving
//! - `bypass`: Direct LLM query (no retrieval)

mod bypass;
mod global;
mod hybrid;
mod local;
mod mix;
mod naive;

use crate::error::Result;
use crate::types::{QueryMode, QueryParams, QueryResult};
use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use std::sync::Arc;

/// Engine for executing RAG queries.
pub struct QueryEngine {
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
}

impl QueryEngine {
    /// Create a new query engine.
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
        graph_storage: Arc<dyn GraphStorage>,
        vector_storage: Arc<dyn VectorStorage>,
    ) -> Self {
        Self {
            llm,
            embedding,
            graph_storage,
            vector_storage,
        }
    }

    /// Check if metadata matches tenant context.
    pub(super) fn matches_tenant(
        &self,
        metadata: &serde_json::Value,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> bool {
        // If no tenant context is set, allow all
        if tenant_id.is_none() {
            return true;
        }

        let metadata_map = match metadata.as_object() {
            Some(map) => map,
            None => return true, // No metadata, allow for backward compatibility
        };

        // Check if properties have matching tenant_id
        if let Some(ctx_tenant_id) = tenant_id {
            if let Some(prop_tenant_id) = metadata_map.get("tenant_id").and_then(|v| v.as_str()) {
                if prop_tenant_id != ctx_tenant_id {
                    return false;
                }
            }
            // If no tenant_id in properties but context has one, still include for backward compatibility
        }

        // Check workspace_id if set
        if let Some(ctx_workspace_id) = workspace_id {
            if let Some(prop_workspace_id) =
                metadata_map.get("workspace_id").and_then(|v| v.as_str())
            {
                if prop_workspace_id != ctx_workspace_id {
                    return false;
                }
            }
        }

        true
    }

    /// Execute a query.
    pub async fn query(&self, query: &str, params: QueryParams) -> Result<QueryResult> {
        let start = std::time::Instant::now();

        let result = match params.mode {
            QueryMode::Naive => self.query_naive(query, &params).await?,
            QueryMode::Local => self.query_local(query, &params).await?,
            QueryMode::Global => self.query_global(query, &params).await?,
            QueryMode::Mix => self.query_mix(query, &params).await?,
            QueryMode::Hybrid => self.query_hybrid(query, &params).await?,
            QueryMode::Bypass => self.query_bypass(query, &params).await?,
        };

        let mut final_result = result;
        final_result.stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(final_result)
    }
}
