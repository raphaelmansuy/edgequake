//! SPEC-065 — single SSOT for HNSW / filtered-ANN runtime knobs (DRY env reads).

use std::sync::Once;

use super::capabilities::VectorStorageMode;
use crate::filter_column_policy::env_flag_true;

/// Runtime search + index-shape policy for pgvector HNSW (SPEC-064/065).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HnswRuntimePolicy {
    pub storage_mode: VectorStorageMode,
    /// `relaxed_order` | `strict_order` | `off`
    pub iterative_scan_mode: &'static str,
    pub ef_search_override: Option<usize>,
    pub max_scan_tuples: u32,
    pub scan_mem_multiplier: Option<u32>,
    /// Opt-in workspace partial HNSW + column-only filters.
    pub partial_by_workspace: bool,
    /// Force denorm column equality even when partial is off.
    pub columns_only_filters: bool,
    /// Min workspace rows before creating a partial HNSW (shared tables).
    pub partial_min_rows: u64,
}

impl Default for HnswRuntimePolicy {
    fn default() -> Self {
        Self {
            storage_mode: VectorStorageMode::Full,
            iterative_scan_mode: "relaxed_order",
            ef_search_override: None,
            max_scan_tuples: 20_000,
            scan_mem_multiplier: None,
            // IMP-001-01 / July 2026 multitenancy: auto partial HNSW for hot workspaces
            // (still gated by partial_min_rows in ensure_hot_workspace_ann).
            partial_by_workspace: true,
            columns_only_filters: false,
            partial_min_rows: 1_000,
        }
    }
}

impl HnswRuntimePolicy {
    /// Load all knobs from environment (SPEC-065 SSOT).
    ///
    /// `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE`:
    /// - unset / `auto` / `1` / `true` → **on** (IMP-001-01 default for optimal multi-tenant ANN)
    /// - `0` / `false` / `off` → off
    pub fn from_env() -> Self {
        let partial_by_workspace = parse_partial_by_workspace_env(
            &std::env::var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE").unwrap_or_default(),
        );
        let p = Self {
            storage_mode: VectorStorageMode::from_env(),
            iterative_scan_mode: parse_hnsw_iterative_scan_mode(
                &std::env::var("EDGEQUAKE_HNSW_ITERATIVE_SCAN").unwrap_or_default(),
            ),
            ef_search_override: std::env::var("EDGEQUAKE_HNSW_EF_SEARCH")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .map(|n| n.clamp(1, 1000)),
            max_scan_tuples: std::env::var("EDGEQUAKE_HNSW_MAX_SCAN_TUPLES")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(20_000)
                .clamp(1, 2_147_483_647),
            scan_mem_multiplier: std::env::var("EDGEQUAKE_HNSW_SCAN_MEM_MULTIPLIER")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .map(|n| n.clamp(1, 1000)),
            partial_by_workspace,
            columns_only_filters: env_flag_true("EDGEQUAKE_METADATA_FILTER_COLUMNS_ONLY"),
            partial_min_rows: std::env::var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1_000),
        };
        // SPEC-066 / IMP-001-01: discoverability — log once when Wave-2 partial policy is active.
        if p.partial_by_workspace {
            static LOG_ONCE: Once = Once::new();
            LOG_ONCE.call_once(|| {
                tracing::info!(
                    target: "edgequake_storage::hnsw",
                    partial_min_rows = p.partial_min_rows,
                    storage_mode = ?p.storage_mode,
                    "Wave-2 HNSW partial-by-workspace active (default auto; set EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=0 to disable)"
                );
            });
        }
        p
    }

    /// Column-only tenant/workspace predicates (needed for partial index implication).
    pub fn prefer_denorm_filter_columns(&self) -> bool {
        self.partial_by_workspace || self.columns_only_filters
    }
}

/// Resolve HNSW iterative_scan mode from a raw env value.
///
/// IMP-002-01 product contract: default **relaxed_order** (pgvector 0.8.x filtered ANN).
pub fn parse_hnsw_iterative_scan_mode(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "strict" | "strict_order" => "strict_order",
        "off" | "false" | "0" => "off",
        _ => "relaxed_order",
    }
}

/// Parse partial-by-workspace flag (IMP-001-01: default on / auto).
pub fn parse_partial_by_workspace_env(raw: &str) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "0" | "false" | "off" | "no" => false,
        // unset, auto, 1, true, on, empty → enable (threshold still applies)
        _ => true,
    }
}

/// Workspace partial HNSW enabled (default **on** as of IMP-001-01; opt out with `=0`).
pub fn hnsw_partial_by_workspace_enabled() -> bool {
    HnswRuntimePolicy::from_env().partial_by_workspace
}

/// IMP-002-01: product contract for filtered ANN GUC set.
/// Returns true when statements include iterative_scan (or iterative unsupported / off intentionally).
pub fn filtered_ann_gucs_satisfy_contract(
    stmts: &[String],
    iterative_scan_supported: bool,
) -> bool {
    if !iterative_scan_supported {
        return true; // pre-0.8 floor — capabilities gate should block elsewhere
    }
    let joined = stmts.join("; ").to_ascii_lowercase();
    // If operator forced off, contract is "explicit off" (documented recall risk).
    if joined.contains("iterative_scan = off") || joined.contains("iterative_scan=off") {
        return true;
    }
    joined.contains("iterative_scan") && joined.contains("max_scan_tuples")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iterative_scan_modes() {
        assert_eq!(parse_hnsw_iterative_scan_mode(""), "relaxed_order");
        assert_eq!(parse_hnsw_iterative_scan_mode("STRICT"), "strict_order");
        assert_eq!(parse_hnsw_iterative_scan_mode("false"), "off");
        assert_eq!(parse_hnsw_iterative_scan_mode("garbage"), "relaxed_order");
    }

    #[test]
    fn parse_partial_default_auto_on() {
        assert!(parse_partial_by_workspace_env(""));
        assert!(parse_partial_by_workspace_env("auto"));
        assert!(parse_partial_by_workspace_env("1"));
        assert!(!parse_partial_by_workspace_env("0"));
        assert!(!parse_partial_by_workspace_env("off"));
    }

    #[test]
    fn filtered_ann_contract_requires_iterative_and_max_tuples() {
        let ok = vec![
            "SET LOCAL hnsw.ef_search = 40".into(),
            "SET LOCAL hnsw.iterative_scan = relaxed_order".into(),
            "SET LOCAL hnsw.max_scan_tuples = 20000".into(),
        ];
        assert!(filtered_ann_gucs_satisfy_contract(&ok, true));
        assert!(!filtered_ann_gucs_satisfy_contract(
            &["SET LOCAL hnsw.ef_search = 40".into()],
            true
        ));
    }

    #[test]
    fn from_env_clamps_and_flags() {
        std::env::set_var("EDGEQUAKE_HNSW_EF_SEARCH", "99999");
        std::env::set_var("EDGEQUAKE_HNSW_MAX_SCAN_TUPLES", "0");
        std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "1");
        std::env::set_var("EDGEQUAKE_METADATA_FILTER_COLUMNS_ONLY", "yes");
        std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS", "500");
        let p = HnswRuntimePolicy::from_env();
        assert_eq!(p.ef_search_override, Some(1000));
        assert_eq!(p.max_scan_tuples, 1); // clamp min
        assert!(p.partial_by_workspace);
        assert!(p.columns_only_filters);
        assert!(p.prefer_denorm_filter_columns());
        assert_eq!(p.partial_min_rows, 500);
        std::env::remove_var("EDGEQUAKE_HNSW_EF_SEARCH");
        std::env::remove_var("EDGEQUAKE_HNSW_MAX_SCAN_TUPLES");
        std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
        std::env::remove_var("EDGEQUAKE_METADATA_FILTER_COLUMNS_ONLY");
        std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS");
    }
}
