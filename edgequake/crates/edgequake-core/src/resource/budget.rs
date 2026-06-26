//! Resource budget configuration — SPEC-006 SSOT.
//!
//! Single source of truth for caps documented in
//! `specifications/006-ensure-perf/004_resource_budget_catalog.md`.

use serde::{Deserialize, Serialize};

/// Maximum graph nodes returned per API graph request.
pub const MAX_GRAPH_NODES: usize = 500;

/// Maximum graph traversal depth.
pub const MAX_GRAPH_DEPTH: usize = 5;

/// Maximum API page size for list endpoints.
pub const MAX_PAGE_SIZE: usize = 100;

/// Minimum API page size.
pub const MIN_PAGE_SIZE: usize = 1;

/// Default upload / body size limit (50 MiB).
pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

/// Default max query string length.
pub const MAX_QUERY_CHARS: usize = 10_000;

/// Node count above which full-graph scans are rejected at admission.
pub const DEFAULT_GRAPH_SCAN_THRESHOLD_NODES: usize = 50_000;

/// Max concurrent in-process full-graph materializations.
///
/// WHY 4 (was 1): derived from the default DB pool budget, not a guess.
/// Each materialization holds up to 3 concurrent DB connections (the
/// `tokio::join!` of node_count_fast + edge_count_fast +
/// get_popular_nodes_with_degree in `graph_stream.rs`). With the default
/// pool of 32 and 8 connections reserved for non-graph traffic (frontend
/// polling, admin), the safe ceiling is ⌊(32 - 8) / 3⌋ = 8. We pick 4 as
/// the default — half the ceiling — to leave headroom for ingestion
/// workers (which hold connections for the full embedding duration) while
/// still absorbing the 2–3 request interactive burst from React StrictMode
/// double-mount, tenant/workspace switch, and manual refetch.
///
/// The env clamp in `from_env()` is pool-aware: it derives the upper bound
/// from `DATABASE_POOL_SIZE` so graph traffic alone can never exhaust the
/// pool. Operators who raise the pool get a proportionally higher cap.
pub const DEFAULT_GRAPH_MATERIALIZE_CONCURRENT: usize = 4;

/// Fraction of cgroup memory reserved as headroom (0.0–1.0).
pub const DEFAULT_MEM_HEADROOM_RATIO: f64 = 0.75;

/// Graph popular-nodes query timeout (seconds).
pub const DEFAULT_GRAPH_QUERY_TIMEOUT_SECS: u64 = 15;

/// Max concurrent vision PDF conversions process-wide (P-G13 OOM guard).
///
/// WHY 2: each job may spawn multiple parallel page renders (see
/// `EDGEQUAKE_PDF_CONCURRENCY`). Capping document-level jobs prevents
/// N workers × page concurrency from exhausting process memory.
pub const DEFAULT_PDF_VISION_JOBS_CONCURRENT: usize = 2;

/// Orchestrator + SOTA context token cap (RB-LLM-004 / RB-LLM-008 alignment).
pub const MAX_ORCHESTRATOR_CONTEXT_TOKENS: usize = 30_000;

/// Central resource budget — SPEC-006 Layer A (DRY).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudgetConfig {
    pub max_graph_nodes: usize,
    pub max_graph_depth: usize,
    pub max_page_size: usize,
    pub max_upload_bytes: usize,
    pub max_query_chars: usize,
    pub graph_scan_threshold_nodes: usize,
    pub graph_materialize_concurrent: usize,
    pub pdf_vision_jobs_concurrent: usize,
    pub mem_headroom_ratio: f64,
    pub graph_query_timeout_secs: u64,
}

impl Default for ResourceBudgetConfig {
    fn default() -> Self {
        Self {
            max_graph_nodes: MAX_GRAPH_NODES,
            max_graph_depth: MAX_GRAPH_DEPTH,
            max_page_size: MAX_PAGE_SIZE,
            max_upload_bytes: MAX_UPLOAD_BYTES,
            max_query_chars: MAX_QUERY_CHARS,
            graph_scan_threshold_nodes: DEFAULT_GRAPH_SCAN_THRESHOLD_NODES,
            graph_materialize_concurrent: DEFAULT_GRAPH_MATERIALIZE_CONCURRENT,
            pdf_vision_jobs_concurrent: DEFAULT_PDF_VISION_JOBS_CONCURRENT,
            mem_headroom_ratio: DEFAULT_MEM_HEADROOM_RATIO,
            graph_query_timeout_secs: DEFAULT_GRAPH_QUERY_TIMEOUT_SECS,
        }
    }
}

impl ResourceBudgetConfig {
    /// Load overrides from environment with documented clamps.
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(v) = std::env::var("EDGEQUAKE_GRAPH_SCAN_THRESHOLD") {
            if let Ok(n) = v.parse::<usize>() {
                config.graph_scan_threshold_nodes = n.max(1_000);
            }
        }
        // WHY the clamp is pool-aware: each graph materialization holds up to
        // 3 concurrent DB connections (the `tokio::join!` of node_count_fast +
        // edge_count_fast + get_popular_nodes_with_degree in graph_stream.rs).
        // The cap must therefore be ⌊pool_size / 3⌋ so graph traffic alone can
        // never exhaust the pool. We read DATABASE_POOL_SIZE (default 32) and
        // reserve 8 connections for non-graph traffic (frontend polling, admin),
        // giving an effective ceiling of ⌊(pool - 8) / 3⌋. The absolute upper
        // bound is 8 (24 connections) so a misconfigured huge pool doesn't open
        // an unreasonable number of concurrent heavy graph queries.
        let db_pool_size: usize = std::env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(32);
        let pool_derived_max = db_pool_size.saturating_sub(8) / 3;
        let materialize_max = pool_derived_max.clamp(1, 8);
        if let Ok(v) = std::env::var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT") {
            if let Ok(n) = v.parse::<usize>() {
                config.graph_materialize_concurrent = n.clamp(1, materialize_max);
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                config.graph_query_timeout_secs = n.clamp(1, 120);
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_MAX_UPLOAD_BYTES") {
            if let Ok(n) = v.parse::<usize>() {
                config.max_upload_bytes = n.max(1024 * 1024);
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_PDF_VISION_JOBS") {
            if let Ok(n) = v.parse::<usize>() {
                config.pdf_vision_jobs_concurrent = n.clamp(1, 8);
            }
        }
        config
    }

    /// Clamp list page size — BR-006-010.
    #[inline]
    pub fn clamp_page_size(&self, page_size: u32) -> u32 {
        page_size.clamp(MIN_PAGE_SIZE as u32, self.max_page_size as u32)
    }

    /// Clamp graph query max_nodes.
    #[inline]
    pub fn clamp_graph_max_nodes(&self, max_nodes: usize) -> usize {
        max_nodes.clamp(1, self.max_graph_nodes)
    }

    /// Clamp graph traversal depth.
    #[inline]
    pub fn clamp_graph_depth(&self, depth: usize) -> usize {
        depth.clamp(1, self.max_graph_depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-006: BR-006-012 — defaults must match catalog.
    #[test]
    fn resource_budget_defaults_match_catalog() {
        let budget = ResourceBudgetConfig::default();
        assert_eq!(budget.max_graph_nodes, 500);
        assert_eq!(budget.max_graph_depth, 5);
        assert_eq!(budget.max_page_size, 100);
        assert_eq!(budget.max_upload_bytes, 50 * 1024 * 1024);
        assert_eq!(budget.max_query_chars, 10_000);
        assert_eq!(budget.graph_scan_threshold_nodes, 50_000);
        assert_eq!(budget.graph_materialize_concurrent, 4);
        assert_eq!(budget.pdf_vision_jobs_concurrent, 2);
        assert_eq!(budget.graph_query_timeout_secs, 15);
    }

    #[test]
    fn clamp_page_size_bounds() {
        let budget = ResourceBudgetConfig::default();
        assert_eq!(budget.clamp_page_size(0), 1);
        assert_eq!(budget.clamp_page_size(200), 100);
        assert_eq!(budget.clamp_page_size(50), 50);
    }

    /// SPEC-006 RB-LLM-008: orchestrator must match SOTA 30k cap.
    #[test]
    fn orchestrator_context_tokens_align_with_sota() {
        assert_eq!(MAX_ORCHESTRATOR_CONTEXT_TOKENS, 30_000);
    }

    /// SPEC-021 R1: the graph materialization clamp must be pool-aware.
    /// Each materialization holds 3 DB connections, so the cap must be
    /// ⌊(pool - 8) / 3⌋ and never exceed 8, regardless of what the operator
    /// requests — otherwise graph traffic alone could exhaust the pool.
    #[test]
    fn graph_materialize_concurrent_clamp_is_pool_aware() {
        // Default pool (32): ⌊(32-8)/3⌋ = 8, requested 16 → clamped to 8.
        std::env::set_var("DATABASE_POOL_SIZE", "32");
        std::env::set_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT", "16");
        let budget = ResourceBudgetConfig::from_env();
        assert_eq!(budget.graph_materialize_concurrent, 8);
        std::env::remove_var("DATABASE_POOL_SIZE");
        std::env::remove_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT");

        // Small pool (14): ⌊(14-8)/3⌋ = 2, requested 8 → clamped to 2.
        std::env::set_var("DATABASE_POOL_SIZE", "14");
        std::env::set_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT", "8");
        let budget = ResourceBudgetConfig::from_env();
        assert_eq!(budget.graph_materialize_concurrent, 2);
        std::env::remove_var("DATABASE_POOL_SIZE");
        std::env::remove_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT");

        // Tiny pool (8): ⌊(8-8)/3⌋ = 0 → clamp(1, max=1) → 1 (floor).
        std::env::set_var("DATABASE_POOL_SIZE", "8");
        std::env::set_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT", "4");
        let budget = ResourceBudgetConfig::from_env();
        assert_eq!(budget.graph_materialize_concurrent, 1);
        std::env::remove_var("DATABASE_POOL_SIZE");
        std::env::remove_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT");

        // Huge pool (200): ⌊(200-8)/3⌋ = 64 → clamp(1, 8) → 8 (absolute cap).
        std::env::set_var("DATABASE_POOL_SIZE", "200");
        std::env::set_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT", "64");
        let budget = ResourceBudgetConfig::from_env();
        assert_eq!(budget.graph_materialize_concurrent, 8);
        std::env::remove_var("DATABASE_POOL_SIZE");
        std::env::remove_var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT");
    }
}
