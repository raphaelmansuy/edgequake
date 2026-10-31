//! SQL WHERE clause builder for [`MetadataFilter`] (SPEC-017 STORE-DRY-001).
//!
//! Encodes the same semantics as [`MetadataFilter::matches`] for postgres pgvector queries.

use crate::filter_column_policy::prefer_denorm_filter_columns;
use crate::traits::MetadataFilter;

/// Dynamic SQL fragments for filtered vector search.
#[derive(Debug, Clone)]
pub struct MetadataFilterSql {
    /// SQL conditions joined with AND (without leading WHERE).
    pub conditions: Vec<String>,
    /// Next bind parameter index after building conditions.
    pub next_param: u32,
}

impl MetadataFilter {
    /// Build SQL conditions mirroring the in-memory [`Self::matches`] predicate.
    ///
    /// Parameter `$1` is reserved for the query embedding vector.
    /// `start_param` is the first bind slot for filters (typically `2`).
    /// Pass `table_alias` (e.g. `"v"`) when the query uses a table alias.
    pub fn build_sql(&self, has_id_filter: bool, start_param: u32) -> MetadataFilterSql {
        self.build_sql_with_alias(has_id_filter, start_param, None)
    }

    /// Same as [`Self::build_sql`] with optional qualified column prefix.
    pub fn build_sql_with_alias(
        &self,
        has_id_filter: bool,
        start_param: u32,
        table_alias: Option<&str>,
    ) -> MetadataFilterSql {
        let q = |col: &str| match table_alias {
            Some(a) => format!("{a}.{col}"),
            None => col.to_string(),
        };

        let mut conditions = Vec::new();
        let mut param_offset = start_param;

        if has_id_filter {
            conditions.push(format!("{} = ANY(${param_offset}::text[])", q("id")));
            param_offset += 1;
        }

        if self.document_ids.is_some() {
            conditions.push(format!(
                "({doc_id} = ANY(${p}::text[]) OR {meta}->>'document_id' = ANY(${p}::text[]) OR {meta}->>'source_document_id' = ANY(${p}::text[]))",
                doc_id = q("document_id"),
                meta = q("metadata"),
                p = param_offset
            ));
            param_offset += 1;
        }

        // SPEC-064: partial HNSW `WHERE workspace_id = $ws` is only usable when the
        // query implies that predicate. The legacy OR JSONB fallback blocks implication.
        // Opt-in via EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE / EDGEQUAKE_METADATA_FILTER_COLUMNS_ONLY.
        let columns_only = prefer_denorm_filter_columns();

        if self.tenant_id.is_some() {
            if columns_only {
                conditions.push(format!("{} = ${param_offset}", q("tenant_id")));
            } else {
                conditions.push(format!(
                    "({tenant} = ${p} OR {meta}->>'tenant_id' = ${p})",
                    tenant = q("tenant_id"),
                    meta = q("metadata"),
                    p = param_offset
                ));
            }
            param_offset += 1;
        }

        if self.workspace_id.is_some() {
            if columns_only {
                conditions.push(format!("{} = ${param_offset}", q("workspace_id")));
            } else {
                conditions.push(format!(
                    "({workspace} = ${p} OR {meta}->>'workspace_id' = ${p})",
                    workspace = q("workspace_id"),
                    meta = q("metadata"),
                    p = param_offset
                ));
            }
            param_offset += 1;
        }

        if self.vector_type.is_some() {
            conditions.push(format!("{}->>'type' = ${param_offset}", q("metadata")));
            param_offset += 1;
        }

        if self.modalities.is_some() {
            conditions.push(format!(
                "{}->>'modality' = ANY(${param_offset}::text[])",
                q("metadata")
            ));
            param_offset += 1;
        }

        MetadataFilterSql {
            conditions,
            next_param: param_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_only_omits_jsonb_or_for_workspace() {
        std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "1");
        let mf = MetadataFilter {
            workspace_id: Some("ws-a".into()),
            tenant_id: Some("t1".into()),
            document_ids: None,
            vector_type: None,
            modalities: None,
            embedding_model: None,
            kinds: None,
        };
        let sql = mf.build_sql(false, 2);
        assert!(sql.conditions.iter().any(|c| c == "workspace_id = $3"));
        assert!(sql.conditions.iter().any(|c| c == "tenant_id = $2"));
        assert!(!sql.conditions.iter().any(|c| c.contains("metadata")));
        std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
    }

    #[test]
    fn build_sql_matches_predicate_fields() {
        let mf = MetadataFilter {
            document_ids: Some(vec!["doc-a".into()]),
            tenant_id: Some("t1".into()),
            workspace_id: Some("ws1".into()),
            vector_type: Some("chunk".into()),
            modalities: Some(vec!["chart".into()]),
            embedding_model: None,
            kinds: None,
        };
        let sql = mf.build_sql(true, 2);
        assert_eq!(sql.conditions.len(), 6);
        assert!(sql.conditions[0].contains("ANY($2"));
        assert!(sql.conditions[1].contains("document_id"));
        assert!(sql.conditions[4].contains("metadata->>'type'"));
        assert!(sql
            .conditions
            .iter()
            .any(|c| c.contains("metadata->>'modality'")));
        assert_eq!(sql.next_param, 8);
    }

    #[test]
    fn empty_filter_yields_no_conditions() {
        let mf = MetadataFilter::default();
        let sql = mf.build_sql(false, 2);
        assert!(sql.conditions.is_empty());
        assert_eq!(sql.next_param, 2);
    }
}
