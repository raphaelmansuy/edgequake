//! Standalone pgvector projection adapter contract.
//!
//! The layout owns only projection keys, vectors, and filter payloads. It
//! never joins the relational authority's chunks or documents.

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, ScopedVectorSearch, VectorSearchPage, VectorSearchRequest,
};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandaloneEmbeddingCapabilities {
    pub requires_relational_colocation: bool,
    pub storage_layout_version: u32,
}

#[derive(Clone)]
pub struct PgStandaloneEmbeddingStore {
    pool: PgPool,
}

impl PgStandaloneEmbeddingStore {
    pub const LAYOUT_VERSION: u32 = 1;

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub const fn capabilities(&self) -> StandaloneEmbeddingCapabilities {
        StandaloneEmbeddingCapabilities {
            requires_relational_colocation: false,
            storage_layout_version: Self::LAYOUT_VERSION,
        }
    }
}

#[async_trait]
impl ScopedVectorSearch for PgStandaloneEmbeddingStore {
    async fn search(&self, _request: &VectorSearchRequest) -> AccessResult<VectorSearchPage> {
        Err(AccessError::UnsupportedCapability(
            "standalone pgvector search awaits authoritative hydration port wiring".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_layout_does_not_require_relational_colocation() {
        assert!(
            !StandaloneEmbeddingCapabilities {
                requires_relational_colocation: false,
                storage_layout_version: PgStandaloneEmbeddingStore::LAYOUT_VERSION,
            }
            .requires_relational_colocation
        );

        let migration =
            include_str!("../../../../../migrations/151_standalone_embedding_projections.sql");
        assert!(!migration.contains("REFERENCES chunks"));
        assert!(!migration.contains("REFERENCES documents"));
    }
}
