//! SQL WHERE clause builder for [`MetadataFilter`] (SPEC-017 STORE-DRY-001).
//!
//! Encodes the same semantics as [`MetadataFilter::matches`] for postgres pgvector queries.

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

        if self.tenant_id.is_some() {
            conditions.push(format!(
                "({tenant} = ${p} OR {meta}->>'tenant_id' = ${p})",
                tenant = q("tenant_id"),
                meta = q("metadata"),
                p = param_offset
            ));
            param_offset += 1;
        }

        if self.workspace_id.is_some() {
            conditions.push(format!(
                "({workspace} = ${p} OR {meta}->>'workspace_id' = ${p})",
                workspace = q("workspace_id"),
                meta = q("metadata"),
                p = param_offset
            ));
            param_offset += 1;
        }

        if self.vector_type.is_some() {
            conditions.push(format!("{}->>'type' = ${param_offset}", q("metadata")));
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
    fn build_sql_matches_predicate_fields() {
        let mf = MetadataFilter {
            document_ids: Some(vec!["doc-a".into()]),
            tenant_id: Some("t1".into()),
            workspace_id: Some("ws1".into()),
            vector_type: Some("chunk".into()),
        };
        let sql = mf.build_sql(true, 2);
        assert_eq!(sql.conditions.len(), 5);
        assert!(sql.conditions[0].contains("ANY($2"));
        assert!(sql.conditions[1].contains("document_id"));
        assert!(sql.conditions[4].contains("metadata->>'type'"));
        assert_eq!(sql.next_param, 7);
    }

    #[test]
    fn empty_filter_yields_no_conditions() {
        let mf = MetadataFilter::default();
        let sql = mf.build_sql(false, 2);
        assert!(sql.conditions.is_empty());
        assert_eq!(sql.next_param, 2);
    }
}
