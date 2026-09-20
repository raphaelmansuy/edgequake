//! Shared helpers for pipeline processing stages.
//!
//! Split by responsibility (SPEC-017 / PIPE-SOLID-S-001):
//! - [`stats`]: extraction linkage and statistics aggregation
//! - [`embeddings`]: embedding generation with token-budget batching
//! - [`unique_embed`]: within-doc unique keys before entity/rel embed (SPEC-047 P6)
//! - [`lineage`]: document lineage construction

mod embeddings;
mod lineage;
pub mod mention_merge;
mod stats;
pub mod unique_embed;

pub(super) use stats::aggregate_extraction_stats;
pub(super) use stats::link_extractions_to_chunks;
