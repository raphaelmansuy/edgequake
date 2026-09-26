//! Document-scoped source lineage SQL predicates (SPEC-021 P-A3 / SPEC-045).
//!
//! SSOT for matching AGE node/edge properties against a document chunk prefix.
//! Two-path design (issue #305/#309):
//! - **Modern**: GIN-friendly `source_ids @>` exact candidates (indexed).
//! - **Legacy**: bounded LIKE/unnest only when modern arrays are absent.
//!
//! # SPEC-089 / GH-336
//!
//! Cartesian `prefixes × SOURCE_CHUNK_PROBE_LIMIT` probes are safe only for
//! **page-scoped** batches. Always combine with [`SOURCE_PREFIX_BATCH_LIMIT`]
//! and [`SOURCE_COUNT_STATEMENT_TIMEOUT_MS`] (LAW-H1 / LAW-H2).

/// Max chunk indices probed for GIN `@>` entity-count reconcile (list hot path).
///
/// WHY: `LIKE '%…%'` / `jsonb_array_elements_text` forces a Seq Scan over
/// `_ag_label_vertex` (~140k+ nodes → multi-second Documents list). Exact
/// chunk-id containment uses `idx_*_source_ids_gin` (~1 ms). Documents with
/// more than this many chunks still get a correct lower bound; stats write
/// path (P-A1) remains the primary count source (LAW-H5).
pub(in crate::adapters::postgres::graph) const SOURCE_CHUNK_PROBE_LIMIT: usize = 256;

/// Max document prefixes per count SQL round-trip (SPEC-089 / GH-336 / LAW-H1).
///
/// WHY: one query with thousands of prefixes × 256 probes saturates the pool
/// even when each GIN probe is cheap (~7ms). Callers chunk larger lists.
///
/// Public SSOT for list analytics **and** StorageInspector INV-C (SPEC-107 R2).
pub const SOURCE_PREFIX_BATCH_LIMIT: usize = 32;

/// Server-side kill for entity-count reconcile (SPEC-089 / GH-336 / LAW-H2).
///
/// WHY: `tokio::time::timeout` alone abandons the Rust future while Postgres
/// keeps running (zombie pool holders). `SET LOCAL statement_timeout` inside
/// a transaction cancels the statement; aligned under API `AGE_RECONCILE_TIMEOUT`
/// (400ms).
///
/// Public SSOT for list analytics **and** StorageInspector INV-C (SPEC-107 R2).
pub const SOURCE_COUNT_STATEMENT_TIMEOUT_MS: u32 = 300;

/// Server-side kill for cascade discovery GIN probes (SPEC-089 Wave 3 / F-336-08).
///
/// WHY: delete/reprocess use the same `CROSS JOIN generate_series` CTE as counts
/// but need a larger budget than list reconcile (typical ~100ms; concurrent
/// storms must still die ≪ minutes).
pub(in crate::adapters::postgres::graph) const SOURCE_DISCOVERY_STATEMENT_TIMEOUT_MS: u32 = 2000;

/// Server-side kill for workspace dashboard AGE counts (SPEC-089 Phase 4 / F-336-14).
///
/// WHY: `GET /workspaces/{id}/stats` wraps fetch in `tokio::timeout(4s)`. PG must
/// cancel first (LAW-H2) — 250ms headroom under the app budget.
pub(in crate::adapters::postgres::graph) const WORKSPACE_STATS_STATEMENT_TIMEOUT_MS: u32 = 3_750;

/// GIN pending-list cap (kB) for lineage indexes (SPEC-149 / Migration 157).
///
/// WHY: every discovery probe scans the unsorted pending list. At the 4 MB
/// default (~512 pages) a probe costs ~1 ms, so 257 probes × 2 arms blow the
/// discovery budget; at 256 kB (~32 pages) probes stay sub-0.1 ms.
pub const LINEAGE_GIN_PENDING_LIST_LIMIT_KB: u32 = 256;

/// Lineage GIN indexes probed by discovery / counts (one per indexed key × label).
pub const LINEAGE_GIN_INDEXES: [&str; 4] = [
    "idx_node_source_ids_gin",
    "idx_node_source_chunk_ids_gin",
    "idx_edge_source_ids_gin",
    "idx_edge_source_chunk_ids_gin",
];

/// `WITH (...)` storage clause for lineage GIN `CREATE INDEX` DDL.
pub(in crate::adapters::postgres::graph) fn lineage_gin_storage_clause() -> String {
    format!("WITH (gin_pending_list_limit = {LINEAGE_GIN_PENDING_LIST_LIMIT_KB})")
}

/// Resolve probe series upper bound from known chunk counts (SPEC-089).
///
/// `0` / unknown → full [`SOURCE_CHUNK_PROBE_LIMIT`]. Otherwise
/// `clamp(max_chunk_count, 1..=SOURCE_CHUNK_PROBE_LIMIT)` so
/// `generate_series(0, n-1)` covers chunk indices `0..chunk_count-1`.
pub(in crate::adapters::postgres::graph) fn source_count_probe_limit(
    max_chunk_count: usize,
) -> usize {
    if max_chunk_count == 0 {
        SOURCE_CHUNK_PROBE_LIMIT
    } else {
        max_chunk_count.clamp(1, SOURCE_CHUNK_PROBE_LIMIT)
    }
}

use super::escape::escape_sql_literal;

pub use crate::lineage_canon::INDEXED_LINEAGE_ARRAY_KEYS;

/// SSOT probe CTE for cascade discovery (IMP-031-08).
///
/// # Planner law (2026-07-25 incident)
///
/// Putting tenant/workspace predicates on the same join as
/// `source_ids @> probe` lets Postgres prefer `idx_node_tenant_id` (~30k
/// rows) then recheck `@>` as a **Join Filter** (~4s @ 200k nodes → 15s
/// `statement_timeout` on batch delete).
///
/// **Probe-first** + **`MATERIALIZED`** forces Nested Loop from probes →
/// `Bitmap Index Scan on idx_*_source_ids_gin` (~100ms).
///
/// `$1` = exact ids, `$2` = chunk prefixes, `$3` = probe series upper bound.
pub(in crate::adapters::postgres::graph) fn source_ids_probes_cte_sql() -> &'static str {
    r#"
            probes AS MATERIALIZED (
              SELECT probe_id FROM unnest($1::text[]) AS t(probe_id)
              UNION
              SELECT (p.prefix || gs.i::text) AS probe_id
              FROM unnest($2::text[]) AS p(prefix)
              CROSS JOIN generate_series(0, $3::int - 1) AS gs(i)
            )
    "#
}

/// Count-path prefixes CTE: `$1` = `{doc}-chunk-` prefixes, `$2` = series upper.
///
/// Probes every chunk id plus the bare document id, the same token set as
/// discovery ([`source_ids_probes_cte_sql`]), so counts match the scoped graph.
pub(in crate::adapters::postgres::graph) fn source_ids_count_probes_cte_sql() -> &'static str {
    r#"
            prefixes AS MATERIALIZED (
              SELECT prefix, ord
              FROM unnest($1::text[]) WITH ORDINALITY AS t(prefix, ord)
            ),
            probes AS MATERIALIZED (
              SELECT p.prefix, p.ord, (p.prefix || gs.i::text) AS chunk_id
              FROM prefixes p
              CROSS JOIN generate_series(0, $2::int - 1) AS gs(i)
              UNION ALL
              SELECT p.prefix, p.ord, left(p.prefix, -length('-chunk-')) AS chunk_id
              FROM prefixes p
              WHERE right(p.prefix, length('-chunk-')) = '-chunk-'
            )
    "#
}

/// Probe-first `hits AS MATERIALIZED (...)` body: UNION of GIN joins on every
/// indexed lineage array (`source_ids` and `source_chunk_ids`).
///
/// `from_join` is the shared `FROM probes pr INNER JOIN {graph}."Label" alias`
/// prefix without the ON clause. `select_list` is the SELECT column list
/// (must be identical across UNION arms). `extra_where` is optional
/// `WHERE …` appended to each arm (may be empty).
///
/// WHY UNION (not OR): each arm independently uses
/// `idx_*_source_ids_gin` / `idx_*_source_chunk_ids_gin`, and hashing the
/// properties blobs once in the UNION beat an OR + DISTINCT plan ~3× on a
/// 120k-node live graph (SPEC-149). Counts use OR instead — see
/// [`lineage_count_hits_cte_sql`].
pub(in crate::adapters::postgres::graph) fn lineage_hits_cte_sql(
    select_list: &str,
    from_join: &str,
    props: &str,
    extra_where: &str,
) -> String {
    let arms: Vec<String> = INDEXED_LINEAGE_ARRAY_KEYS
        .iter()
        .map(|key| {
            format!(
                "SELECT {select_list} {from_join} \
                 ON (({props})::jsonb -> '{key}') @> to_jsonb(pr.probe_id) \
                 {extra_where}"
            )
        })
        .collect();
    format!(
        "hits AS MATERIALIZED (\n              {}\n            )",
        arms.join("\n              UNION\n              ")
    )
}

/// Count-path hits: one probe join whose ON ORs every indexed lineage key.
///
/// `from_join` includes `FROM probes pr INNER JOIN … alias` without ON.
/// `select_list` must include `pr.prefix, pr.ord` plus a stable row id.
///
/// WHY OR (not UNION like discovery): the parameterized probe join plans as
/// a `BitmapOr` over both GIN indexes, so each heap row is rechecked once.
/// UNION arms recheck every mirrored row twice — ~2× the cost on counts,
/// which select only ids and have no properties blob to dedup.
pub(in crate::adapters::postgres::graph) fn lineage_count_hits_cte_sql(
    select_list: &str,
    from_join: &str,
    props: &str,
) -> String {
    let predicates: Vec<String> = INDEXED_LINEAGE_ARRAY_KEYS
        .iter()
        .map(|key| format!("(({props})::jsonb -> '{key}') @> to_jsonb(pr.chunk_id)"))
        .collect();
    format!(
        "hits AS MATERIALIZED (\n              SELECT {select_list} {from_join} ON ({})\n            )",
        predicates.join(" OR ")
    )
}

/// Batched per-prefix node counts over both indexed lineage arrays.
///
/// Shared by `analytics_ops` and StorageInspector INV-C (SPEC-107 R2) so the
/// two can never diverge. Binds: `$1` = `{doc}-chunk-` prefixes (text[]),
/// `$2` = probe series upper bound (int). Returns `(prefix, cnt BIGINT)` in
/// input order, one row per prefix (0 when unmatched).
pub fn node_counts_by_source_prefixes_sql(graph_name: &str) -> String {
    let probes_cte = source_ids_count_probes_cte_sql();
    let hits_cte = lineage_count_hits_cte_sql(
        "pr.prefix, pr.ord, v.id",
        &format!(r#"FROM probes pr INNER JOIN {graph_name}."Node" v"#),
        "ag_catalog.agtype_to_json(v.properties)",
    );
    format!(
        r#"
            /* DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES */
            WITH {probes_cte},
            {hits_cte}
            SELECT p.prefix, count(DISTINCT h.id)::BIGINT AS cnt
            FROM prefixes p
            LEFT JOIN hits h ON h.prefix = p.prefix
            GROUP BY p.prefix, p.ord
            ORDER BY p.ord
            "#,
        probes_cte = probes_cte.trim(),
        hits_cte = hits_cte.trim(),
    )
}

/// Normalize a document / chunk prefix to the `{doc_id}-chunk-` form.
///
/// Accepts either a bare document id or an already-suffixed
/// [`crate::kv_keys::doc_chunk_prefix`] value.
pub(in crate::adapters::postgres::graph) fn normalize_doc_chunk_prefix(prefix: &str) -> String {
    if prefix.ends_with("-chunk-") {
        prefix.to_string()
    } else {
        format!("{prefix}-chunk-")
    }
}

/// Build concrete chunk-id candidates for GIN `@>` probes (`{prefix}0`..`N-1`).
///
/// Batched list counts use SQL `generate_series` instead; this helper remains
/// for single-prefix probes, scan predicates, and tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::adapters::postgres::graph) fn source_chunk_id_candidates(
    prefix: &str,
    limit: usize,
) -> Vec<String> {
    let chunk_prefix = normalize_doc_chunk_prefix(prefix);
    let n = limit.clamp(1, SOURCE_CHUNK_PROBE_LIMIT);
    (0..n).map(|i| format!("{chunk_prefix}{i}")).collect()
}

/// Modern indexed path: GIN `@>` on every [`INDEXED_LINEAGE_ARRAY_KEYS`] entry.
///
/// WHY (deletion timeout / #305): OR-ing hundreds of `@>` probes with unindexed
/// LIKE/unnest forces Nested Loop Seq Scan. Keep this helper GIN-only on the
/// indexed lineage arrays. Discovery hot paths in `scan_ops` use
/// [`lineage_hits_cte_sql`] (UNION of GIN joins) instead of giant OR trees.
pub(in crate::adapters::postgres::graph) fn jsonb_matches_doc_source_prefix_modern(
    props: &str,
    doc_prefix: &str,
) -> String {
    let esc = escape_sql_literal(doc_prefix);
    let chunk = escape_sql_literal(&normalize_doc_chunk_prefix(doc_prefix));
    let mut parts = Vec::new();
    for key in INDEXED_LINEAGE_ARRAY_KEYS {
        parts.push(format!("({props}->'{key}') @> to_jsonb('{esc}'::text)"));
        for i in 0..SOURCE_CHUNK_PROBE_LIMIT {
            parts.push(format!(
                "({props}->'{key}') @> to_jsonb(('{chunk}' || '{i}')::text)"
            ));
        }
    }
    format!("({})", parts.join(" OR "))
}

/// Legacy-only path: pipe `source_id` / LIKE / unnest when modern arrays are absent.
///
/// Bounded to rows without usable indexed lineage arrays so wipe-all never needs
/// this and cascade discovery does not SeqScan the whole modern graph.
pub(in crate::adapters::postgres::graph) fn jsonb_matches_doc_source_prefix_legacy(
    props: &str,
    doc_prefix: &str,
) -> String {
    let esc = escape_sql_literal(doc_prefix);
    let chunk = escape_sql_literal(&normalize_doc_chunk_prefix(doc_prefix));
    format!(
        "((jsonb_typeof({props}->'source_ids') IS DISTINCT FROM 'array' \
          OR jsonb_array_length(COALESCE({props}->'source_ids', '[]'::jsonb)) = 0) \
         AND (jsonb_typeof({props}->'source_chunk_ids') IS DISTINCT FROM 'array' \
          OR jsonb_array_length(COALESCE({props}->'source_chunk_ids', '[]'::jsonb)) = 0) \
         AND ( \
           {props}->>'source_id' = '{esc}' \
           OR {props}->>'source_id' LIKE '{esc}%' \
           OR {props}->>'source_id' LIKE '%|{esc}%' \
           OR {props}->>'source_id' LIKE '%|{chunk}%' \
           OR {props}->>'source_id' LIKE '{chunk}%' \
         ))",
        props = props,
        esc = esc,
        chunk = chunk,
    )
}

/// Combined predicate (compat / unit tests). Prefer two-path queries in `scan_ops`.
#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::adapters::postgres::graph) fn jsonb_matches_doc_source_prefix(
    props: &str,
    doc_prefix: &str,
) -> String {
    format!(
        "({} OR {})",
        jsonb_matches_doc_source_prefix_modern(props, doc_prefix),
        jsonb_matches_doc_source_prefix_legacy(props, doc_prefix),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_path_is_gin_only_on_indexed_lineage_arrays() {
        let sql = jsonb_matches_doc_source_prefix_modern("props", "doc-abc");
        assert!(sql.contains("@>"));
        assert!(!sql.contains("LIKE"));
        assert!(sql.contains("source_ids"));
        assert!(sql.contains("source_chunk_ids"));
        assert!(sql.contains("doc-abc-chunk-"));
        assert!(
            sql.contains("doc-abc-chunk-40") || sql.contains("|| '40'"),
            "modern path must probe past chunk 15: {sql}"
        );
    }

    #[test]
    fn lineage_hits_cte_unions_both_gin_keys() {
        let cte = lineage_hits_cte_sql(
            "v.properties",
            r#"FROM probes pr INNER JOIN g."Node" v"#,
            "ag_catalog.agtype_to_json(v.properties)",
            "",
        );
        assert!(cte.contains("hits AS MATERIALIZED"));
        assert!(cte.contains("UNION"));
        assert!(cte.contains("-> 'source_ids'"));
        assert!(cte.contains("-> 'source_chunk_ids'"));
        assert!(!cte.contains("LIKE"));
    }

    #[test]
    fn lineage_count_hits_cte_ors_both_gin_keys_in_one_join() {
        let cte = lineage_count_hits_cte_sql(
            "pr.prefix, pr.ord, v.id",
            r#"FROM probes pr INNER JOIN g."Node" v"#,
            "ag_catalog.agtype_to_json(v.properties)",
        );
        assert!(!cte.contains("UNION"), "{cte}");
        assert_eq!(cte.matches("INNER JOIN").count(), 1, "{cte}");
        assert!(cte.contains(
            "(ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids') @> to_jsonb(pr.chunk_id) OR"
        ));
        assert!(cte.contains("-> 'source_chunk_ids') @> to_jsonb(pr.chunk_id)"));
    }

    #[test]
    fn legacy_path_requires_empty_indexed_arrays() {
        let sql = jsonb_matches_doc_source_prefix_legacy("props", "doc-abc");
        assert!(sql.contains("LIKE"));
        assert!(sql.contains("jsonb_typeof"));
        assert!(sql.contains("source_chunk_ids"));
        assert!(sql.contains("source_ids"));
    }

    #[test]
    fn includes_source_chunk_ids_array_path() {
        let sql = jsonb_matches_doc_source_prefix("props", "doc-abc");
        assert!(sql.contains("source_chunk_ids"));
        assert!(sql.contains("source_ids"));
        assert!(sql.contains("doc-abc-chunk-"));
    }

    #[test]
    fn normalize_accepts_bare_doc_id_and_chunk_prefix() {
        assert_eq!(normalize_doc_chunk_prefix("doc-abc"), "doc-abc-chunk-");
        assert_eq!(
            normalize_doc_chunk_prefix("doc-abc-chunk-"),
            "doc-abc-chunk-"
        );
    }

    #[test]
    fn chunk_candidates_are_gin_friendly_exact_ids() {
        let ids = source_chunk_id_candidates("doc-abc-chunk-", 3);
        assert_eq!(
            ids,
            vec![
                "doc-abc-chunk-0".to_string(),
                "doc-abc-chunk-1".to_string(),
                "doc-abc-chunk-2".to_string(),
            ]
        );
    }

    #[test]
    fn probe_cte_helpers_are_materialized_and_probe_first() {
        let p = source_ids_probes_cte_sql();
        assert!(p.contains("probes AS MATERIALIZED"));
        assert!(p.contains("unnest($1::text[])"));
        assert!(p.contains("generate_series"));
        let c = source_ids_count_probes_cte_sql();
        assert!(c.contains("prefixes AS MATERIALIZED"));
        assert!(c.contains("probes AS MATERIALIZED"));
        assert!(
            c.contains("left(p.prefix, -length('-chunk-'))"),
            "count probes must include the bare document id like discovery: {c}"
        );
    }

    #[test]
    fn spec089_batch_and_timeout_constants() {
        assert_eq!(SOURCE_PREFIX_BATCH_LIMIT, 32);
        assert_eq!(SOURCE_COUNT_STATEMENT_TIMEOUT_MS, 300);
        assert_eq!(SOURCE_DISCOVERY_STATEMENT_TIMEOUT_MS, 2000);
        assert_eq!(WORKSPACE_STATS_STATEMENT_TIMEOUT_MS, 3_750);
        const {
            assert!(WORKSPACE_STATS_STATEMENT_TIMEOUT_MS < 4_000);
        }
        assert_eq!(SOURCE_CHUNK_PROBE_LIMIT, 256);
        assert_eq!(
            INDEXED_LINEAGE_ARRAY_KEYS,
            ["source_ids", "source_chunk_ids"]
        );
    }

    #[test]
    fn spec089_probe_limit_from_chunk_count() {
        assert_eq!(source_count_probe_limit(0), SOURCE_CHUNK_PROBE_LIMIT);
        assert_eq!(source_count_probe_limit(5), 5);
        assert_eq!(source_count_probe_limit(1), 1);
        assert_eq!(
            source_count_probe_limit(SOURCE_CHUNK_PROBE_LIMIT + 50),
            SOURCE_CHUNK_PROBE_LIMIT
        );
    }
}
