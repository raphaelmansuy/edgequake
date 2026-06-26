//! Query-side caches (SPEC-021 P-G9 / RC-14).
//!
//! Currently provides the embedding cache (`CachingEmbeddingProvider`) that
//! memoizes `embed_one` for the query path. A query-result cache for
//! `context_only` retrieval is tracked as a follow-up (see plan-19 P-G9).

pub mod embedding_cache;

pub use embedding_cache::CachingEmbeddingProvider;
