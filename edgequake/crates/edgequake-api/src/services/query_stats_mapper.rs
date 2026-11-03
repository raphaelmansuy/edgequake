//! Map engine [`edgequake_query::QueryStats`] → HTTP [`QueryStats`] / stream stats.
//!
//! SOLID: single responsibility — one place owns the API stats projection.
//! DRY: `/query`, chat completion, and stream `done` all call these helpers.
//! SPEC-047 W0b: surface `context_empty`, arm timings, and arm chunk counts
//! so harness diagnostics are law (engine fields), not heuristics.
//! SPEC-083 D-40: stream stats are built from the same [`QueryStats`] SSOT.

use crate::handlers::query_types::{QueryStats, QueryStreamStats};
use edgequake_query::{QueryContext, QueryStats as EngineQueryStats};

/// Build HTTP query stats from engine stats + retrieved context sizes.
pub fn from_engine_stats(
    engine: &EngineQueryStats,
    context: &QueryContext,
    llm_provider: Option<String>,
    llm_model: Option<String>,
) -> QueryStats {
    let tokens_used = if engine.generated_tokens > 0 {
        Some(engine.generated_tokens)
    } else {
        None
    };

    let tokens_per_second = if engine.generation_time_ms > 0 && engine.generated_tokens > 0 {
        Some((engine.generated_tokens as f32) / (engine.generation_time_ms as f32) * 1000.0)
    } else {
        None
    };

    QueryStats {
        embedding_time_ms: engine.embedding_time_ms,
        keyword_time_ms: engine.keyword_time_ms,
        retrieval_time_ms: engine.retrieval_time_ms,
        generation_time_ms: engine.generation_time_ms,
        ttft_ms: engine.ttft_ms,
        answer_cache_hit: engine.answer_cache_hit,
        keyword_cache_hit: engine.keyword_cache_hit,
        total_time_ms: engine.total_time_ms,
        sources_retrieved: context.chunks.len()
            + context.entities.len()
            + context.relationships.len(),
        rerank_time_ms: engine.rerank_time_ms,
        tokens_used,
        tokens_per_second,
        llm_provider,
        llm_model,
        context_empty: engine.context_empty,
        context_truncated: engine.context_truncated,
        relevance_dropped: engine.relevance_dropped,
        arm_local_ms: engine.arm_local_ms,
        arm_global_ms: engine.arm_global_ms,
        arm_naive_ms: engine.arm_naive_ms,
        arm_local_chunks: engine.arm_local_chunks,
        arm_global_chunks: engine.arm_global_chunks,
        arm_naive_chunks: engine.arm_naive_chunks,
        arms_run: engine.arms_run.clone(),
        arms_gated: engine.arms_gated,
        query_intent: engine.query_intent.clone(),
    }
}

/// Timing + LLM identity for stream `done` stats (D-40 SSOT; keeps arity low).
pub struct StreamStatsInput {
    pub query_mode: String,
    pub retrieval_time_ms: u64,
    pub generation_time_ms: u64,
    pub ttft_ms: Option<u64>,
    pub ux_ttft_ms: Option<u64>,
    pub tokens_used: u32,
    pub tokens_per_second: Option<f32>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
}

/// Build stream `done` stats from context metadata + timings (D-40 SSOT).
pub fn stream_stats_from_context(
    context: &QueryContext,
    input: StreamStatsInput,
) -> QueryStreamStats {
    let mut engine = EngineQueryStats::default();
    engine.absorb_arm_metadata(context);
    engine.retrieval_time_ms = input.retrieval_time_ms;
    engine.generation_time_ms = input.generation_time_ms;
    engine.ttft_ms = input.ttft_ms;
    engine.total_time_ms = input
        .retrieval_time_ms
        .saturating_add(input.generation_time_ms);
    engine.generated_tokens = input.tokens_used as usize;
    engine.context_empty = context.chunks.is_empty()
        && context.entities.is_empty()
        && context.relationships.is_empty();

    let base = from_engine_stats(&engine, context, input.llm_provider, input.llm_model);
    let mut stream = QueryStreamStats::from_query_stats(
        base,
        input.query_mode,
        input.ux_ttft_ms,
        input.tokens_used,
    );
    stream.tokens_per_second = input.tokens_per_second.or(stream.tokens_per_second);
    stream
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_query::QueryStats as EngineQueryStats;

    #[test]
    fn maps_context_empty_and_arms() {
        let engine = EngineQueryStats {
            context_empty: true,
            arms_run: Some("local,naive".into()),
            arms_gated: Some(true),
            arm_local_ms: Some(11),
            arm_naive_ms: Some(22),
            arm_local_chunks: Some(3),
            arm_naive_chunks: Some(5),
            ..Default::default()
        };
        let ctx = QueryContext::new();
        let stats = from_engine_stats(&engine, &ctx, Some("mistral".into()), Some("small".into()));
        assert!(stats.context_empty);
        assert_eq!(stats.arms_run.as_deref(), Some("local,naive"));
        assert_eq!(stats.arms_gated, Some(true));
        assert_eq!(stats.arm_local_chunks, Some(3));
        assert_eq!(stats.arm_naive_chunks, Some(5));
        assert_eq!(stats.sources_retrieved, 0);
        assert_eq!(stats.llm_provider.as_deref(), Some("mistral"));
    }

    #[test]
    fn contract_stream_stats_superset() {
        use edgequake_query::mix_weights::META_ARMS_RUN;
        let mut ctx = QueryContext::new();
        ctx.metadata
            .insert(META_ARMS_RUN.into(), serde_json::json!("local,global"));
        let stream = stream_stats_from_context(
            &ctx,
            StreamStatsInput {
                query_mode: "mix".into(),
                retrieval_time_ms: 10,
                generation_time_ms: 20,
                ttft_ms: Some(5),
                ux_ttft_ms: Some(15),
                tokens_used: 42,
                tokens_per_second: Some(2100.0),
                llm_provider: Some("ollama".into()),
                llm_model: Some("gemma".into()),
            },
        );
        assert_eq!(stream.query_mode, "mix");
        assert_eq!(stream.ux_ttft_ms, Some(15));
        assert_eq!(stream.tokens_used, 42);
        assert_eq!(stream.arms_run.as_deref(), Some("local,global"));
        assert_eq!(stream.llm_provider.as_deref(), Some("ollama"));
        // Superset: every sync QueryStats diagnostic field has a stream mirror.
        let sync = from_engine_stats(
            &{
                let mut e = EngineQueryStats::default();
                e.absorb_arm_metadata(&ctx);
                e
            },
            &ctx,
            None,
            None,
        );
        assert_eq!(stream.arms_run, sync.arms_run);
        assert_eq!(stream.arm_local_ms, sync.arm_local_ms);
        assert_eq!(stream.arm_global_ms, sync.arm_global_ms);
        assert_eq!(stream.arm_naive_ms, sync.arm_naive_ms);
        assert_eq!(stream.arms_gated, sync.arms_gated);
        assert_eq!(stream.query_intent, sync.query_intent);
        // SPEC-184: the relevance-gate count is mirrored too, so a stream client can tell an
        // "nothing relevant" answer from an empty corpus exactly as the sync client can.
        assert_eq!(stream.relevance_dropped, sync.relevance_dropped);
    }

    #[test]
    fn the_relevance_count_reaches_the_http_stats() {
        // The defect this closes was invisible in production: the answer said "no textual
        // evidence" while twenty pictures sat in the context. A client needs to be able to see
        // that the floor removed them.
        let engine = EngineQueryStats {
            context_empty: true,
            relevance_dropped: Some(20),
            ..Default::default()
        };
        let ctx = QueryContext::new();
        let stats = from_engine_stats(&engine, &ctx, None, None);
        assert!(stats.context_empty);
        assert_eq!(stats.relevance_dropped, Some(20));
    }

    #[test]
    fn a_graph_profile_reports_no_relevance_count() {
        // `None` distinguishes "the gate did not run" from "the gate removed nothing".
        let ctx = QueryContext::new();
        let stats = from_engine_stats(&EngineQueryStats::default(), &ctx, None, None);
        assert_eq!(stats.relevance_dropped, None);
    }
}
