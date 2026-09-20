//! Scoped vector search contracts.

use crate::error::AccessResult;
use crate::ids::{DocumentId, EmbeddingKey};
use crate::scope::AccessScope;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorSearchMode {
    Exact,
    Approximate,
}

/// Full immutable model identity; equal dimensions do not imply compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorModelDescriptor {
    pub provider: String,
    pub model: String,
    pub version: String,
    pub dimensions: u32,
    pub metric: String,
}

/// Complete internal vector request. `Some([])` business filters match nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    pub scope: AccessScope,
    pub model: VectorModelDescriptor,
    pub family: String,
    pub embedding: Vec<f32>,
    pub top_k: u32,
    pub document_ids: Option<Vec<DocumentId>>,
    pub modalities: Option<Vec<String>>,
    pub filter_ids: Option<Vec<Uuid>>,
    pub threshold: Option<f32>,
    pub search_mode: VectorSearchMode,
    pub deadline: DateTime<Utc>,
    pub scan_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearchHit {
    pub key: EmbeddingKey,
    pub subject_id: Uuid,
    pub content_revision: u64,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearchPage {
    pub hits: Vec<VectorSearchHit>,
    pub next_cursor: Option<String>,
    pub budget_exhausted: bool,
}

#[async_trait]
pub trait ScopedVectorSearch: Send + Sync {
    async fn search(&self, request: &VectorSearchRequest) -> AccessResult<VectorSearchPage>;
}
