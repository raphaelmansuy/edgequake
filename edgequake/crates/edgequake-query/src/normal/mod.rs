//! SPEC-180 normal-RAG retrieval: dense cosine + BM25, fused by weighted RRF.
//!
//! A normal workspace has no graph and no keyword LLM, so this module never touches
//! either (LAW-180-6). The pieces are small and separate so each is testable alone:
//!
//! - [`plan`]: what a normal query carries (precomputed vectors, kind filters).
//! - [`weights`]: the two fusion weights and their validated env parsing.
//! - [`fuse`]: weighted RRF with a stable tie-break.
//! - [`retriever`]: the two legs, run concurrently, then fused.
//! - [`rewrite`]: optional query expansion before either leg.
//! - [`mmr`]: near-duplicate suppression after rerank.

pub mod fuse;
pub mod mmr;
pub mod plan;
pub mod retriever;
pub mod rewrite;
pub mod weights;

pub use fuse::{fuse_ranked, FusedHit};
pub use plan::{NormalQueryPlan, QueryModality};
pub use retriever::{NormalOutcome, NormalSearch, NormalTrace};
pub use weights::NormalWeights;

/// Context metadata keys written by [`retriever`] (retrieval trace, no chunk text).
pub mod meta {
    pub const PROFILE: &str = "normal_profile";
    pub const DENSE_HITS: &str = "normal_dense_hits";
    pub const BM25_HITS: &str = "normal_bm25_hits";
    pub const FUSED_HITS: &str = "normal_fused_hits";
    pub const BM25_BACKEND: &str = "normal_bm25_backend";
    pub const QUERY_MODALITY: &str = "normal_query_modality";
    /// Chunks MMR removed as near-copies. `0` when MMR did not change the list.
    pub const MMR_DROPPED: &str = "normal_mmr_dropped";
    /// `true` when retrieval used a rewritten question.
    pub const QUERY_REWRITTEN: &str = "normal_query_rewritten";
    /// Wall time of the rewrite call, in milliseconds (0 when it did not run).
    pub const REWRITE_MS: &str = "normal_rewrite_ms";
    /// LLM calls spent on rewriting. Absorbed into `QueryStats.llm_calls`.
    pub const REWRITE_LLM_CALLS: &str = "normal_rewrite_llm_calls";
}
