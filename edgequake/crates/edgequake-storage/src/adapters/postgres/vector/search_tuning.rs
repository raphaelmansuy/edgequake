//! Search tuning and embedding serialization for [`super::PgVectorStorage`].

use super::super::config::VectorIndexType;
use super::PgVectorStorage;

impl PgVectorStorage {
    pub(crate) fn format_embedding(embedding: &[f32]) -> String {
        let values: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        format!("[{}]", values.join(","))
    }

    pub(crate) fn search_tuning_statements(
        index_type: VectorIndexType,
        top_k: usize,
        filtered: bool,
        iterative_scan_supported: bool,
    ) -> Vec<String> {
        let mut stmts = Vec::new();
        match index_type {
            VectorIndexType::HNSW => {
                let ef = (top_k.saturating_mul(4)).clamp(40, 1000);
                stmts.push(format!("SET LOCAL hnsw.ef_search = {}", ef));
                if filtered && iterative_scan_supported {
                    stmts.push("SET LOCAL hnsw.iterative_scan = strict_order".to_string());
                    stmts.push("SET LOCAL hnsw.max_scan_tuples = 20000".to_string());
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
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 10, true, true);
        assert!(stmts.iter().any(|s| s.contains("hnsw.ef_search")));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.iterative_scan = strict_order"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.max_scan_tuples = 20000"));
    }

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
