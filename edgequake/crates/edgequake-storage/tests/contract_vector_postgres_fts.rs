//! SPEC-023 I10 — Postgres native FTS on vector chunk content.

#[test]
fn contract_postgres_vector_fts_joins_shared_kv_for_chunk_text() {
    let fts = include_str!("../src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("ts_rank_cd"));
    assert!(fts.contains("websearch_to_tsquery"));
    assert!(fts.contains("chunk_kv_table_name"));
    assert!(fts.contains("k.value->>'content'"));
    assert!(fts.contains("LEFT JOIN"));
    assert!(fts.contains("content_tsv"));
}

#[test]
fn contract_workspace_vector_uses_shared_chunk_kv_table() {
    let ws = include_str!("../src/adapters/postgres/workspace_vector.rs");
    assert!(
        ws.contains("qualified_kv_table"),
        "workspace vectors must join the shared default KV for FTS"
    );
}

#[test]
fn contract_vector_ddl_adds_content_tsv() {
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    assert!(ddl.contains("content_tsv"));
    assert!(ddl.contains("ensure_content_fts"));
}
