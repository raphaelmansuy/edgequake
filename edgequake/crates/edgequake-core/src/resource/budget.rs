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
pub const DEFAULT_GRAPH_MATERIALIZE_CONCURRENT: usize = 1;

/// Fraction of cgroup memory reserved as headroom (0.0–1.0).
pub const DEFAULT_MEM_HEADROOM_RATIO: f64 = 0.75;

/// Graph popular-nodes query timeout (seconds).
pub const DEFAULT_GRAPH_QUERY_TIMEOUT_SECS: u64 = 15;

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
        if let Ok(v) = std::env::var("EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT") {
            if let Ok(n) = v.parse::<usize>() {
                config.graph_materialize_concurrent = n.clamp(1, 16);
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
        assert_eq!(budget.graph_materialize_concurrent, 1);
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
}
