//! SPEC-023 I10 — Postgres native FTS on vector chunk content.

#[test]
fn contract_postgres_vector_fts_joins_kv_for_chunk_text() {
    let fts = include_str!("../src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("ts_rank_cd"));
    assert!(fts.contains("websearch_to_tsquery"));
    assert!(fts.contains("k.value->>'content'"));
    assert!(fts.contains("LEFT JOIN"));
}

#[test]
fn contract_vector_ddl_adds_content_tsv() {
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    assert!(ddl.contains("content_tsv"));
    assert!(ddl.contains("ensure_content_fts"));
}
