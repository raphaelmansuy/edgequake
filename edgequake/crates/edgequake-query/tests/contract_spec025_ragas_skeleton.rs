//! SPEC-025 8.1 — RAGAS-style eval skeleton contract.

use edgequake_query::eval::{
    context_entity_recall, keyword_recall_in_text, load_spec025_golden_set, GoldenSetStats,
};

#[test]
fn contract_spec025_golden_set_has_fifty_cases() {
    let cases = load_spec025_golden_set();
    let stats = GoldenSetStats::from_cases(&cases);
    assert_eq!(stats.case_count, 50);
    assert!(
        stats.with_context_entities >= 50,
        "each case should declare expected context entities"
    );
}

#[test]
fn contract_spec025_eval_metrics_run_on_golden_sample() {
    let cases = load_spec025_golden_set();
    let sample = &cases[0];

    let answer_score = keyword_recall_in_text(
        "ENTITY_01 is a mock entity used in regression tests",
        &sample.expected_answer_keywords,
    );
    assert!(
        answer_score > 0.0,
        "keyword recall metric must score plausible answers"
    );

    let context_score = context_entity_recall(
        &["ENTITY_01".to_string(), "OTHER".to_string()],
        &sample.expected_context_entities,
    );
    assert!(
        (context_score - 1.0).abs() < f32::EPSILON,
        "context entity recall must reach 1.0 when entity present"
    );
}
