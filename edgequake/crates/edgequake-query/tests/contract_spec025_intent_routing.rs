//! SPEC-025 6.4 — cheap intent routing contract.

use edgequake_query::keywords::QueryIntent;
use edgequake_query::modes::QueryMode;

#[test]
fn contract_spec025_intent_routing_avoids_triple_arm_for_exploratory() {
    assert_eq!(
        QueryIntent::Exploratory.recommended_mode(),
        QueryMode::Naive,
        "exploratory must not route to Hybrid/Mix"
    );
}

#[test]
fn contract_spec025_intent_routing_mix_only_for_procedural() {
    let expensive = [QueryMode::Mix, QueryMode::Hybrid];
    for intent in [
        QueryIntent::Factual,
        QueryIntent::Relational,
        QueryIntent::Exploratory,
        QueryIntent::Comparative,
    ] {
        let mode = intent.recommended_mode();
        assert!(
            !expensive.contains(&mode),
            "{intent} must not use Mix/Hybrid, got {mode:?}"
        );
    }
    assert_eq!(QueryIntent::Procedural.recommended_mode(), QueryMode::Mix);
}
