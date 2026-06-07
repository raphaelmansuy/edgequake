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
    pub fn build_sql(&self, has_id_filter: bool, start_param: u32) -> MetadataFilterSql {
        let mut conditions = Vec::new();
        let mut param_offset = start_param;

        if has_id_filter {
            conditions.push(format!("id = ANY(${param_offset}::text[])"));
            param_offset += 1;
        }

        if self.document_ids.is_some() {
            conditions.push(format!(
                "(document_id = ANY(${p}::text[]) OR metadata->>'document_id' = ANY(${p}::text[]) OR metadata->>'source_document_id' = ANY(${p}::text[]))",
                p = param_offset
            ));
            param_offset += 1;
        }

        if self.tenant_id.is_some() {
            conditions.push(format!(
                "(tenant_id = ${p} OR metadata->>'tenant_id' = ${p})",
                p = param_offset
            ));
            param_offset += 1;
        }

        if self.workspace_id.is_some() {
            conditions.push(format!(
                "(workspace_id = ${p} OR metadata->>'workspace_id' = ${p})",
                p = param_offset
            ));
            param_offset += 1;
        }

        if self.vector_type.is_some() {
            conditions.push(format!("metadata->>'type' = ${param_offset}"));
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
