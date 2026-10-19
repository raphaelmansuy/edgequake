//! ANN-first statements for the unfiltered typed vector searches.
//!
//! The legacy statements join the hydrate table (`entities`, `relationships`,
//! `chunks`) *inside* the `ORDER BY distance LIMIT k` block. Under forced row
//! security that join is planned from the hydrate table's policy estimates,
//! which cannot see the workspace skew behind `workspace_id = (SELECT …)`.
//! The planner then bitmap-scans a whole tenant (32k entities) to answer a
//! query that needs 180 neighbours: 370 ms for entities and 510 ms for
//! relationships, against 8-11 ms when the HNSW scan stands alone.
//!
//! First principles: an ordered index scan can only serve `ORDER BY dist
//! LIMIT k` on the table that owns the vector. So the distance order runs on
//! that table alone, inside a `MATERIALIZED` CTE, and the names are hydrated
//! for the `k` survivors afterwards.
//!
//! This is only equivalent when the hydrate join does not filter, so these
//! builders serve the plain statements. Document, modality and id filters live
//! on the hydrate table, must run before the limit, and keep their legacy
//! statements. The optional predicates stay in the hydrate join as always-NULL
//! guards so every `$n` the caller binds is still referenced and typed.
//!
//! Lossless because the embedding rows cascade-delete with their parent and
//! carry the parent's `workspace_id` (verified by `e2e_query_audit`).

use super::document_lineage_sql;

/// Plain entity ANN. Binds: `$1` vector, `$2` model, `$3` workspace, `$4`
/// documents, `$5` tenant, `$6` modalities, `$7` limit.
pub(super) fn entity_sql(dim: i32, cast: &str) -> String {
    let guard = document_lineage_sql::entity_guard();
    format!(
        "WITH ann AS MATERIALIZED ( \
            SELECT fe.entity_id, \
                   1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
            FROM entity_embeddings fe \
            WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
              AND ($3::uuid IS NULL OR fe.workspace_id = $3) \
            ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $7) \
         SELECT ('entity:' || e.name) AS legacy_id, ann.score AS score \
         FROM ann \
         JOIN entities e ON e.id = ann.entity_id \
         WHERE {guard} \
           AND ($5::uuid IS NULL OR e.tenant_id = $5) \
           AND ($6::text[] IS NULL OR e.modality = ANY($6))"
    )
}

/// Plain relationship ANN. Binds: `$1` vector, `$2` model, `$3` workspace,
/// `$4` documents, `$5` tenant, `$6` modalities, `$7` ids, `$8` limit.
pub(super) fn relationship_sql(dim: i32, cast: &str) -> String {
    let guard = document_lineage_sql::relationship_guard();
    format!(
        "WITH ann AS MATERIALIZED ( \
            SELECT fe.relationship_id, \
                   1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
            FROM relationship_embeddings fe \
            WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
              AND ($3::uuid IS NULL OR fe.workspace_id = $3) \
            ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $8) \
         SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                ann.score AS score \
         FROM ann \
         JOIN relationships r ON r.id = ann.relationship_id \
         JOIN entities es ON es.id = r.source_id \
         JOIN entities et ON et.id = r.target_id \
         WHERE $6::text[] IS NULL \
           AND {guard} \
           AND ($5::uuid IS NULL OR r.tenant_id = $5) \
           AND ($7::text[] IS NULL OR r.id::text = ANY($7) \
                OR (es.name || '->' || et.name || ':' || r.relation_type) = ANY($7))"
    )
}

/// Plain chunk ANN. Binds: `$1` vector, `$2` model, `$3` workspace, `$4`
/// documents, `$5` tenant, `$6` modalities, `$7` vector type, `$8` limit.
pub(super) fn chunk_sql(dim: i32, cast: &str) -> String {
    format!(
        "WITH ann AS MATERIALIZED ( \
            SELECT ce.chunk_id, \
                   1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score \
            FROM chunk_embeddings ce \
            WHERE ce.model_id = $2 AND ce.dimensions = {dim} \
              AND ($3::uuid IS NULL OR ce.workspace_id = $3) \
            ORDER BY (ce.embedding::{cast}) <=> $1::{cast} LIMIT $8) \
         SELECT ann.chunk_id AS chunk_id, ann.score AS score \
         FROM ann \
         JOIN chunks c ON c.id = ann.chunk_id \
         WHERE ($4::uuid[] IS NULL OR c.document_id = ANY($4)) \
           AND ($5::uuid IS NULL OR c.tenant_id = $5) \
           AND ($6::text[] IS NULL OR c.modality = ANY($6)) \
           AND ($7::text IS NULL OR lower($7) = 'chunk')"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bind the caller sends must be referenced, or Postgres cannot
    /// infer its type and rejects the statement.
    fn references_binds(sql: &str, count: usize) {
        for n in 1..=count {
            assert!(sql.contains(&format!("${n}")), "missing ${n} in {sql}");
        }
        assert!(
            !sql.contains(&format!("${}", count + 1)),
            "extra bind in {sql}"
        );
    }

    /// The distance order and its limit run before any join.
    fn orders_before_join(sql: &str) {
        let order = sql.find("ORDER BY").expect("ORDER BY");
        let join = sql.find("JOIN").expect("JOIN");
        let limit = sql.find("LIMIT").expect("LIMIT");
        assert!(order < limit && limit < join, "{sql}");
        assert!(sql.starts_with("WITH ann AS MATERIALIZED"));
        assert_eq!(sql.matches("ORDER BY").count(), 1, "{sql}");
    }

    #[test]
    fn entity_statement_orders_the_embeddings_alone() {
        let sql = entity_sql(1024, "halfvec(1024)");
        references_binds(&sql, 7);
        orders_before_join(&sql);
        assert!(sql.contains("fe.dimensions = 1024"));
    }

    #[test]
    fn relationship_statement_orders_the_embeddings_alone() {
        let sql = relationship_sql(1024, "halfvec(1024)");
        references_binds(&sql, 8);
        orders_before_join(&sql);
        assert!(sql.contains("JOIN entities es") && sql.contains("JOIN entities et"));
    }

    #[test]
    fn chunk_statement_orders_the_embeddings_alone() {
        let sql = chunk_sql(768, "halfvec(768)");
        references_binds(&sql, 8);
        orders_before_join(&sql);
        assert!(sql.contains("ce.dimensions = 768"));
    }

    #[test]
    fn hydrate_joins_keep_optional_guards_after_the_limit() {
        for sql in [
            entity_sql(1024, "halfvec(1024)"),
            relationship_sql(1024, "halfvec(1024)"),
            chunk_sql(1024, "halfvec(1024)"),
        ] {
            let limit = sql.find("LIMIT").unwrap();
            assert!(sql[limit..].contains("$4::uuid[] IS NULL"), "{sql}");
            assert!(sql[limit..].contains("$5::uuid IS NULL"), "{sql}");
        }
    }
}
