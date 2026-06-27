//! SPEC-026 Phase 4j — surrounding context contract (LightRAG `test_multimodal_surrounding_context.py`).

use edgequake_api::services::{
    build_surrounding, find_target_span, load_chunk_separators, SurroundingContext,
    SurroundingKind, SurroundingTokenCounter,
};

#[test]
fn find_target_span_table_with_id_anywhere_in_attrs() {
    let content = r#"before <table format="json" id="tb-abcd-0007">[[1,2],[3,4]]</table> after"#;
    let span = find_target_span(SurroundingKind::Tables, "tb-abcd-0007", content).unwrap();
    let snippet = &content[span.0..span.1];
    assert!(snippet.ends_with("</table>"));
    assert!(snippet.contains("tb-abcd-0007"));
}

#[test]
fn find_target_span_table_cite_marker() {
    let content = r#"before <cite type="table" refid="tb-abcd-0007">表1</cite> after"#;
    let span = find_target_span(SurroundingKind::Tables, "tb-abcd-0007", content).unwrap();
    assert!(content[span.0..span.1].starts_with("<cite"));
}

#[test]
fn drawing_surrounding_kept_within_block() {
    let block = concat!(
        "paragraph one ends. paragraph two. ",
        r#"<drawing id="im-1" path="a.png" src="a" /> "#,
        "paragraph three. paragraph four."
    );
    let span = find_target_span(SurroundingKind::Drawings, "im-1", block).unwrap();
    let surr = build_surrounding(
        SurroundingKind::Drawings,
        block,
        span,
        2000,
        2000,
        &load_chunk_separators(),
        SurroundingTokenCounter::Char,
    );
    assert!(surr.leading.ends_with("paragraph two. "));
    assert!(surr.trailing.starts_with(" paragraph three."));
}

#[test]
fn table_surrounding_strips_sibling_tables() {
    let block = concat!(
        r#"<table id="tb-other" format="json">[["a","b"],["c","d"]]</table> "#,
        "narrative text describing the report. ",
        r#"<table id="tb-target" format="json">[["x","y"]]</table>"#,
        " concluding remarks."
    );
    let span = find_target_span(SurroundingKind::Tables, "tb-target", block).unwrap();
    let surr = build_surrounding(
        SurroundingKind::Tables,
        block,
        span,
        2000,
        2000,
        &load_chunk_separators(),
        SurroundingTokenCounter::Char,
    );
    assert!(!surr.leading.contains("<table"));
    assert!(surr.leading.contains("narrative text"));
    assert!(surr.trailing.contains("concluding remarks"));
}

#[test]
fn from_item_wires_token_budget_surrounding() {
    let md = concat!(
        "intro. ",
        r#"<table id="tb-1" format="html"><tr><td>A</td></tr></table>"#,
        " outro."
    );
    let span = find_target_span(SurroundingKind::Tables, "tb-1", md).unwrap();
    std::env::set_var("EDGEQUAKE_MM_SURROUNDING_TOKENS", "char");
    let ctx = SurroundingContext::from_span(md, span, SurroundingKind::Tables);
    assert!(ctx.leading.contains("intro"));
    assert!(ctx.trailing.contains("outro"));
    std::env::remove_var("EDGEQUAKE_MM_SURROUNDING_TOKENS");
}
