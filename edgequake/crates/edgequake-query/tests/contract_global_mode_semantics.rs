//! SPEC-023 I2 — global mode documentation matches implementation.

#[test]
fn contract_global_mode_docs_do_not_claim_community_reports() {
    let src = include_str!("../src/modes.rs");
    assert!(
        !src.contains("Community-based search using graph clusters"),
        "Global mode rustdoc must not claim GraphRAG community clusters"
    );
    assert!(
        src.contains("Not") && src.contains("GraphRAG"),
        "Global mode must explicitly disclaim GraphRAG community reports"
    );
}
