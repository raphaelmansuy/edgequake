//! Shared helpers for pipeline processing stages.
//!
//! Split by responsibility (SPEC-017 / PIPE-SOLID-S-001):
//! - [`stats`]: extraction linkage and statistics aggregation
//! - [`embeddings`]: embedding generation with token-budget batching
//! - [`lineage`]: document lineage construction

mod embeddings;
mod lineage;
mod stats;

pub(super) use stats::{aggregate_extraction_stats, link_extractions_to_chunks};
