//! Search tuning and embedding serialization for [`super::PgVectorStorage`].

use super::super::config::VectorIndexType;
use super::super::hnsw_runtime_policy::{parse_hnsw_iterative_scan_mode, HnswRuntimePolicy};
use super::PgVectorStorage;
use crate::traits::MetadataFilter;

impl PgVectorStorage {
    pub(crate) fn format_embedding(embedding: &[f32]) -> String {
        let values: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        format!("[{}]", values.join(","))
    }

    /// SPEC-067: session-local planner bias toward HNSW / partial when Wave-2 shape applies.
    ///
    /// Only when columns-only filters (partial implication) and no JSONB OR shapes
    /// (`document_ids` / `modalities`). Does **not** drop global HNSW.
    ///
    /// SPEC-080 B3: when `workspace_row_count` is `Some(n)` and `n <= ann_exact_max_rows()`,
    /// skip bias so the planner may choose exact (seq/btree) on tiny workspace slices.
    /// Public for SPEC-080 contracts (pure; no DB).
    pub fn wave2_planner_bias_statements(
        prefer_columns: bool,
        partial_ready: bool,
        mf: &MetadataFilter,
        workspace_row_count: Option<u64>,
    ) -> Vec<String> {
        if !prefer_columns || !partial_ready {
            return Vec::new();
        }
        let jsonb_or_shapes = mf.document_ids.as_ref().is_some_and(|v| !v.is_empty())
            || mf.modalities.as_ref().is_some_and(|v| !v.is_empty());
        if jsonb_or_shapes || mf.workspace_id.is_none() {
            return Vec::new();
        }
        if let Some(n) = workspace_row_count {
            if n <= crate::filter_column_policy::ann_exact_max_rows() {
                return Vec::new();
            }
        }
        vec![
            "SET LOCAL enable_seqscan = off".to_string(),
            "SET LOCAL random_page_cost = 1.1".to_string(),
        ]
    }

    /// Build session-local ANN GUCs (SPEC-054/075). Public for contract tests.
    pub fn search_tuning_statements(
        index_type: VectorIndexType,
        top_k: usize,
        filtered: bool,
        iterative_scan_supported: bool,
    ) -> Vec<String> {
        let policy = HnswRuntimePolicy::from_env();
        Self::search_tuning_statements_with_overrides(
            index_type,
            top_k,
            filtered,
            iterative_scan_supported,
            policy.iterative_scan_mode,
            policy.ef_search_override,
            policy.max_scan_tuples,
            policy.scan_mem_multiplier,
        )
    }

    /// Pure variant for tests / DI (SPEC-046 OPS-P1.5 — no env reads for mode string).
    pub fn search_tuning_statements_with_hnsw_mode(
        index_type: VectorIndexType,
        top_k: usize,
        filtered: bool,
        iterative_scan_supported: bool,
        hnsw_iterative_mode: &str,
    ) -> Vec<String> {
        let policy = HnswRuntimePolicy::from_env();
        Self::search_tuning_statements_with_overrides(
            index_type,
            top_k,
            filtered,
            iterative_scan_supported,
            hnsw_iterative_mode,
            policy.ef_search_override,
            policy.max_scan_tuples,
            policy.scan_mem_multiplier,
        )
    }

    /// Pure GUC builder (SPEC-064 Wave 3 — injectable overrides for battle grid / tests).
    #[allow(clippy::too_many_arguments)] // injectable override grid mirrors HnswRuntimePolicy fields
    pub(crate) fn search_tuning_statements_with_overrides(
        index_type: VectorIndexType,
        top_k: usize,
        filtered: bool,
        iterative_scan_supported: bool,
        hnsw_iterative_mode: &str,
        ef_search_override: Option<usize>,
        max_scan_tuples: u32,
        scan_mem_multiplier: Option<u32>,
    ) -> Vec<String> {
        let mut stmts = Vec::new();
        match index_type {
            VectorIndexType::HNSW => {
                let ef = ef_search_override
                    .unwrap_or_else(|| (top_k.saturating_mul(4)).clamp(40, 1000))
                    .clamp(1, 1000);
                stmts.push(format!("SET LOCAL hnsw.ef_search = {}", ef));
                if filtered && iterative_scan_supported {
                    // IMP-002-01 / pgvector 0.8.x: iterative scans are the product contract
                    // for filtered ANN (post-filter under-K without them).
                    let mode = parse_hnsw_iterative_scan_mode(hnsw_iterative_mode);
                    if mode != "off" {
                        stmts.push(format!("SET LOCAL hnsw.iterative_scan = {}", mode));
                        stmts.push(format!(
                            "SET LOCAL hnsw.max_scan_tuples = {}",
                            max_scan_tuples.clamp(1, 2_147_483_647)
                        ));
                        if let Some(mult) = scan_mem_multiplier {
                            stmts.push(format!(
                                "SET LOCAL hnsw.scan_mem_multiplier = {}",
                                mult.clamp(1, 1000)
                            ));
                        }
                    } else {
                        tracing::warn!(
                            target: "edgequake_storage::hnsw",
                            "filtered ANN with hnsw.iterative_scan=off — recall under-K risk (IMP-002-01)"
                        );
                    }
                }
            }
            VectorIndexType::IVFFlat => {
                let probes = top_k.clamp(10, 200);
                stmts.push(format!("SET LOCAL ivfflat.probes = {}", probes));
                if filtered && iterative_scan_supported {
                    stmts.push("SET LOCAL ivfflat.iterative_scan = relaxed_order".to_string());
                }
            }
            VectorIndexType::None => {}
        }
        stmts
    }

    pub(crate) async fn supports_iterative_scan(&self) -> bool {
        *self
            .iterative_scan_supported
            .get_or_init(|| async {
                let pool = match self.pool.get().await {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                let version: Option<(String,)> =
                    sqlx::query_as("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
                        .fetch_optional(&pool)
                        .await
                        .ok()
                        .flatten();
                match version {
                    Some((v,)) => {
                        let supported = pgvector_supports_iterative_scan(&v);
                        tracing::debug!(
                            pgvector_version = %v,
                            iterative_scan_supported = supported,
                            "Detected pgvector iterative-scan capability"
                        );
                        supported
                    }
                    None => false,
                }
            })
            .await
    }

    pub(crate) fn parse_embedding(text: &str) -> Vec<f32> {
        let trimmed = text.trim_start_matches('[').trim_end_matches(']');
        trimmed
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect()
    }
}

/// Return true if pgvector `extversion` is >= 0.8.0 (iterative-scan GUCs).
pub(crate) fn pgvector_supports_iterative_scan(version: &str) -> bool {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok());
    let major = parts.next();
    let minor = parts.next().unwrap_or(0);
    match major {
        Some(0) => minor >= 8,
        Some(_) => true,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::hnsw_runtime_policy::parse_hnsw_iterative_scan_mode;
    use super::*;

    #[test]
    fn test_format_embedding() {
        let embedding = vec![1.0, 2.0, 3.0];
        let formatted = PgVectorStorage::format_embedding(&embedding);
        assert_eq!(formatted, "[1,2,3]");
    }

    #[test]
    fn test_parse_embedding() {
        let text = "[1,2,3]";
        let parsed = PgVectorStorage::parse_embedding(text);
        assert_eq!(parsed, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_search_tuning_hnsw_clamps_ef_search() {
        let small =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 1, false, true);
        assert_eq!(small, vec!["SET LOCAL hnsw.ef_search = 40"]);
        let mid = PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 50, false, true);
        assert_eq!(mid, vec!["SET LOCAL hnsw.ef_search = 200"]);
        let huge =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 100_000, false, true);
        assert_eq!(huge, vec!["SET LOCAL hnsw.ef_search = 1000"]);
    }

    #[test]
    fn test_search_tuning_hnsw_filtered_enables_iterative_scan() {
        let stmts = PgVectorStorage::search_tuning_statements_with_hnsw_mode(
            VectorIndexType::HNSW,
            10,
            true,
            true,
            "relaxed_order",
        );
        assert!(stmts.iter().any(|s| s.contains("hnsw.ef_search")));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.iterative_scan = relaxed_order"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.max_scan_tuples = 20000"));
    }

    #[test]
    fn test_search_tuning_overrides_ef_and_scan_mem() {
        let stmts = PgVectorStorage::search_tuning_statements_with_overrides(
            VectorIndexType::HNSW,
            10,
            true,
            true,
            "relaxed_order",
            Some(120),
            5_000,
            Some(2),
        );
        assert!(stmts.iter().any(|s| s == "SET LOCAL hnsw.ef_search = 120"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.max_scan_tuples = 5000"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.scan_mem_multiplier = 2"));
    }

    #[test]
    fn test_search_tuning_hnsw_iterative_scan_strict() {
        let stmts = PgVectorStorage::search_tuning_statements_with_hnsw_mode(
            VectorIndexType::HNSW,
            10,
            true,
            true,
            "strict_order",
        );
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.iterative_scan = strict_order"));
    }

    #[test]
    fn test_search_tuning_hnsw_iterative_scan_off() {
        let stmts = PgVectorStorage::search_tuning_statements_with_hnsw_mode(
            VectorIndexType::HNSW,
            10,
            true,
            true,
            "off",
        );
        assert!(!stmts.iter().any(|s| s.contains("iterative_scan")));
    }

    #[test]
    fn test_parse_hnsw_iterative_scan_mode_edge_cases() {
        assert_eq!(parse_hnsw_iterative_scan_mode(""), "relaxed_order");
        assert_eq!(parse_hnsw_iterative_scan_mode("STRICT"), "strict_order");
        assert_eq!(
            parse_hnsw_iterative_scan_mode(" relaxed_order "),
            "relaxed_order"
        );
        assert_eq!(parse_hnsw_iterative_scan_mode("false"), "off");
        assert_eq!(parse_hnsw_iterative_scan_mode("garbage"), "relaxed_order");
    }

    // parse_hnsw_iterative_scan_mode is re-exported from hnsw_runtime_policy via super.

    #[test]
    fn test_search_tuning_hnsw_filtered_without_iterative_scan_support() {
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 10, true, false);
        assert!(stmts.iter().any(|s| s.contains("hnsw.ef_search")));
        assert!(!stmts.iter().any(|s| s.contains("iterative_scan")));
        assert!(!stmts.iter().any(|s| s.contains("max_scan_tuples")));
    }

    #[test]
    fn test_search_tuning_ivfflat() {
        let plain =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, false, true);
        assert_eq!(plain, vec!["SET LOCAL ivfflat.probes = 10"]);
        let filtered =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, true, true);
        assert!(filtered
            .iter()
            .any(|s| s == "SET LOCAL ivfflat.iterative_scan = relaxed_order"));
    }

    #[test]
    fn test_search_tuning_ivfflat_without_iterative_scan_support() {
        let filtered =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, true, false);
        assert_eq!(filtered, vec!["SET LOCAL ivfflat.probes = 10"]);
    }

    #[test]
    fn test_search_tuning_none_is_empty() {
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::None, 100, true, true);
        assert!(stmts.is_empty());
    }

    #[test]
    fn test_wave2_planner_bias_columns_only() {
        let mf = MetadataFilter {
            workspace_id: Some("ws-a".into()),
            tenant_id: Some("t1".into()),
            vector_type: Some("chunk".into()),
            document_ids: None,
            modalities: None,
        };
        let stmts = PgVectorStorage::wave2_planner_bias_statements(true, true, &mf, Some(50_000));
        assert!(stmts.iter().any(|s| s == "SET LOCAL enable_seqscan = off"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL random_page_cost = 1.1"));
        // SPEC-080: tiny slice skips bias
        assert!(
            PgVectorStorage::wave2_planner_bias_statements(true, true, &mf, Some(100)).is_empty()
        );
    }

    #[test]
    fn test_wave2_planner_bias_skips_jsonb_or_and_when_not_ready() {
        let with_docs = MetadataFilter {
            workspace_id: Some("ws-a".into()),
            document_ids: Some(vec!["d1".into()]),
            ..Default::default()
        };
        assert!(PgVectorStorage::wave2_planner_bias_statements(
            true,
            true,
            &with_docs,
            Some(50_000)
        )
        .is_empty());
        let plain = MetadataFilter {
            workspace_id: Some("ws-a".into()),
            ..Default::default()
        };
        assert!(
            PgVectorStorage::wave2_planner_bias_statements(false, true, &plain, Some(50_000))
                .is_empty()
        );
        assert!(
            PgVectorStorage::wave2_planner_bias_statements(true, false, &plain, Some(50_000))
                .is_empty()
        );
    }

    #[test]
    fn test_pgvector_version_gate() {
        assert!(pgvector_supports_iterative_scan("0.8.0"));
        assert!(pgvector_supports_iterative_scan("0.8.2"));
        assert!(pgvector_supports_iterative_scan("1.0.0"));
        assert!(pgvector_supports_iterative_scan("0.9"));
        assert!(!pgvector_supports_iterative_scan("0.7.4"));
        assert!(!pgvector_supports_iterative_scan("0.7"));
        assert!(!pgvector_supports_iterative_scan("0.5.1"));
        assert!(!pgvector_supports_iterative_scan(""));
        assert!(!pgvector_supports_iterative_scan("garbage"));
    }
}
