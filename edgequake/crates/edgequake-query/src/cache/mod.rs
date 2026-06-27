//! Query-side caches (SPEC-021 P-G9 / RC-14).
//!
//! - `CachingEmbeddingProvider`: memoizes `embed_one` for the query path.
//! - `QueryResultCache`: memoizes `context_only` retrieval contexts.

pub mod embedding_cache;
pub mod query_result_cache;

pub use embedding_cache::CachingEmbeddingProvider;
pub use query_result_cache::QueryResultCache;

/// Port for invalidating cached `context_only` retrieval after ingestion (DIP).
pub trait QueryResultCacheInvalidator: Send + Sync {
    fn invalidate_query_result_cache(&self);

    /// Workspace-scoped invalidation — does not bust other workspaces' cache entries.
    fn invalidate_query_result_cache_for_workspace(&self, workspace_id: &str) {
        let _ = workspace_id;
        self.invalidate_query_result_cache();
    }
}
