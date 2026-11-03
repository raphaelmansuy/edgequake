//! SPEC-184 — the relevance floor, proved across the layer boundary where the defect lived.
//!
//! The unit tests in `edgequake-query` prove the rule and its effect on the chunk list. This file
//! proves the two things that only become true when the crate boundary is crossed, because *that*
//! is where the original bug hid: retrieval produced the right answer and the layer above it could
//! not see why.
//!
//! 1. The count of dropped chunks survives into the HTTP stats, so a client can tell "this corpus
//!    has nothing on that" from "nothing was retrieved".
//! 2. A query whose pictures are all noise yields an **empty** context, which is what
//!    `context_empty` has always promised and what the model needs to stop explaining itself.

use edgequake_api::handlers::query_types::{QueryStats, QueryStreamStats};
use edgequake_api::services::query_stats_mapper::from_engine_stats;
use edgequake_query::context::RetrievedChunk;
use edgequake_query::{
    normal::{apply_floor, Candidate, MIN_IMAGE_SIMILARITY},
    QueryContext, QueryStats as EngineQueryStats,
};

/// The measured rows from `relevance.rs`, as a candidate list.
fn measured(scores: &[f32]) -> Vec<Candidate> {
    scores
        .iter()
        .map(|s| Candidate {
            similarity: Some(*s),
            is_image: true,
        })
        .collect()
}

#[test]
fn a_picture_corpus_that_knows_nothing_about_the_question_reports_an_honest_empty_context() {
    // Measured: "what does the handbook say" against a 20-photograph corpus.
    let (keep, gate) = apply_floor(&measured(&[0.473, 0.467, 0.466, 0.440]));
    assert_eq!(keep, vec![false, false, false, false]);
    assert!(gate.emptied());

    // The engine now has zero chunks and says so; the HTTP layer must carry both facts.
    let engine = EngineQueryStats {
        context_empty: true,
        relevance_dropped: Some(gate.dropped),
        ..Default::default()
    };
    let stats: QueryStats = from_engine_stats(&engine, &QueryContext::new(), None, None);

    assert!(stats.context_empty, "no noise may be offered as evidence");
    assert_eq!(
        stats.relevance_dropped,
        Some(4),
        "the client must see that the corpus WAS searched"
    );
    assert_eq!(stats.sources_retrieved, 0);
}

#[test]
fn a_matching_picture_query_reports_matches_and_a_small_drop_count() {
    // Measured: "a golden retriever puppy" against the same corpus.
    let (keep, gate) = apply_floor(&measured(&[0.715, 0.610, 0.569, 0.530]));
    assert_eq!(keep, vec![true, true, true, false]);
    assert!(!gate.emptied());

    let engine = EngineQueryStats {
        context_empty: false,
        relevance_dropped: Some(gate.dropped),
        ..Default::default()
    };

    // One surviving chunk reaches the context, so the stat is not empty.
    let mut context = QueryContext::new();
    context.add_chunk(RetrievedChunk::new("p1", "", 0.0164));
    let stats = from_engine_stats(&engine, &context, None, None);
    assert!(!stats.context_empty);
    assert_eq!(stats.relevance_dropped, Some(1));
    assert_eq!(stats.sources_retrieved, 1);
}

#[test]
fn the_floor_is_the_one_the_ui_already_applies() {
    // The grid and the prompt must agree about which pictures are evidence; if these drift apart,
    // the screen shows thirteen pictures under an answer that says it saw none.
    assert_eq!(MIN_IMAGE_SIMILARITY, 0.48);
}

#[test]
fn the_stream_mirror_carries_the_same_count_as_the_sync_response() {
    // SPEC-083 D-40: stream `done` stats are a superset of the sync stats. A streaming client must
    // be able to make the same distinction a sync one can.
    let engine = EngineQueryStats {
        context_empty: true,
        relevance_dropped: Some(20),
        ..Default::default()
    };
    let context = QueryContext::new();
    let sync = from_engine_stats(&engine, &context, None, None);
    let stream = QueryStreamStats::from_query_stats(sync.clone(), "naive".into(), None, 0);
    assert_eq!(stream.relevance_dropped, sync.relevance_dropped);
}

#[test]
fn a_graph_workspace_reports_no_count_because_the_gate_never_ran() {
    let stats = from_engine_stats(
        &EngineQueryStats::default(),
        &QueryContext::new(),
        None,
        None,
    );
    assert_eq!(
        stats.relevance_dropped, None,
        "None means the gate did not run, not that it removed nothing"
    );
}
