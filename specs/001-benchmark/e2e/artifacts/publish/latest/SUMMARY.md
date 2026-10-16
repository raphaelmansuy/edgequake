# SPEC-001 medical-mid SUMMARY

> EQ mix vs LR mix on GraphRAG-Bench (mistral/mistral-small-latest + mistral/mistral-embed) — publishable dual-SUT under matched top-k + L2 retrieval metrics. Not UltraDomain win-rates; Acc is not paper Table-2 comparable unless P0_paper ablation pins are used.

- **valid:** `True`
- **profile:** `ACC_E2OCC_086_v1_lrlike_arms_v2`
- **judge:** `generation_eval`
- **fixture:** `medical_publish_question_ids_v1` (n=200)
- **dataset revision:** `dc3a111e77dbaf8bbaf51ef331f3cfc9b1b5c546`

## Model lineage

- **sut_llm:** `mistral/mistral-small-latest`
- **sut_vision:** `mistral/mistral-small-latest`
- **sut_embed:** `mistral/mistral-embed@1024d`
- **judge_llm:** `mistral/mistral-small-latest`
- **judge_metric_embed:** `mistral-embed`
- **llm_base_url:** `https://api.mistral.ai/v1`
- **judge_base_url:** `https://api.mistral.ai/v1`

## Overall Acc (L0) — Acc = 0.75·F1 + 0.25·cos

| SUT | Acc | F1 | cos |
|-----|-----|----|-----|
| EdgeQuake mix | 0.7525 | 0.6814 | 0.9657 |
| LightRAG mix | 0.7810 | 0.7192 | 0.9665 |
| Δ (EQ − LR) | -0.0285 | -0.0378 | -0.0008 |

- **Δ Acc 95% CI (bootstrap):** [-0.0784, -0.0013] (n=133)

## Retrieval (L2)

| SUT | evidence_recall | context_relevancy |
|-----|-----------------|-------------------|
| EdgeQuake | 0.9222 | 0.4118 |
| LightRAG | 0.9523 | 0.5001 |

## By question_type (EQ Acc)

- **Fact Retrieval:** 0.6533
- **Complex Reasoning:** 0.7479
- **Contextual Summarize:** 0.8351
- **Creative Generation:** 0.7736

## By question_type (LR Acc)

- **Fact Retrieval:** 0.7402
- **Complex Reasoning:** 0.7606
- **Contextual Summarize:** 0.8405
- **Creative Generation:** 0.7827

## Ops

- EQ empty-answer rate: 0.000
- LR empty-answer rate: 0.000
- EQ empty-context rate: 0.000
- LR empty-context rate: 0.000
- EQ query p50/p95 ms: 5417 / 7290
- LR query p50/p95 ms: 1315 / 1625
- ingest wall s: 714.6
- EQ/LR p50 ratio: 4.119 (SLO ≤1.5×: FAIL/WAIVE)
- EQ stage p50 ms: keyword=1235, embed=1652, retrieve=183, generate=2167

## Pins

```json
{
  "edgequake_git_sha": "20c7845c0",
  "dataset_id": "GraphRAG-Bench/GraphRAG-Bench",
  "dataset_revision": "dc3a111e77dbaf8bbaf51ef331f3cfc9b1b5c546",
  "fixture_id": "medical_publish_question_ids_v1",
  "profile_id": "ACC_E2OCC_086_v1_lrlike_arms_v2",
  "llm_provider": "mistral",
  "llm_model": "mistral-small-latest",
  "vision_provider": "mistral",
  "vision_model": "mistral-small-latest",
  "embedding_provider": "mistral",
  "embedding_model": "mistral-embed",
  "embedding_dim": 1024,
  "llm_base_url": "https://api.mistral.ai/v1",
  "judge_provider": "mistral",
  "judge_model": "mistral-small-latest",
  "judge_base_url": "https://api.mistral.ai/v1",
  "judge_embedding_model": "mistral-embed",
  "lineage": {
    "sut_llm": "mistral/mistral-small-latest",
    "sut_vision": "mistral/mistral-small-latest",
    "sut_embed": "mistral/mistral-embed@1024d",
    "judge_llm": "mistral/mistral-small-latest",
    "judge_metric_embed": "mistral-embed",
    "llm_base_url": "https://api.mistral.ai/v1",
    "judge_base_url": "https://api.mistral.ai/v1"
  },
  "judge_temperature": 0.0,
  "judge_acc_factuality_weight": 0.75,
  "judge_embed_backend": "auto",
  "answer_style": "gold",
  "recommended_mistral_judge": "mistral-medium-latest",
  "acc_formula": "0.75*F1 + 0.25*embed_cosine (weights overridable)",
  "acc_lift_note": "Closer Acc under Mistral judge: answer_style=gold (SUT) + judge_model=mistral-medium-latest; keep L2 gates",
  "publish_fairness": true,
  "retrieve_topk": 30,
  "lr_top_k": 30,
  "lr_chunk_top_k": 30,
  "lr_enable_rerank": false,
  "lr_enable_llm_cache": true,
  "eq_max_results": 30,
  "eq_rerank_top_k": 30,
  "eq_enable_rerank": false,
  "graph_walk": "bfs",
  "kg_chunk_pick": "vector",
  "l2_retrieval_required": true,
  "mix_arm_gate": false,
  "eq_mix_arm_gate_env": "false",
  "mix_fusion": "round_robin",
  "rr_order": "local_first",
  "related_chunk_number": 5,
  "kg_chunk_occurrence_sort": true,
  "bm25_retrieval": true,
  "kg_chunk_pick_timing": "per_arm",
  "kg_chunk_pick_lr_budget": true,
  "mix_relevancy_prune": false,
  "mix_relevancy_keep": 12,
  "mix_relevancy_score": "rrf",
  "mix_graph_soft_prune": false,
  "eq_reranker": "bm25",
  "eq_reranker_provider": "",
  "path_prune_fraction": 0.0,
  "path_prune_orphan_entities": false,
  "rerank_protect_first": 0,
  "min_rerank_score": 0.1,
  "entity_rank": "retrieval",
  "relation_select": "default",
  "mix_local_weight": 1.0,
  "mix_global_weight": 1.0,
  "mix_naive_weight": 1.0,
  "context_format": "flat",
  "passage_pack": false,
  "graph_walk_compress": false,
  "popular_node_fallback": false,
  "keyword_lexical_boost": false,
  "content_headings": false,
  "l2_sources_union": false,
  "l2_sources_mix_top_k": 30,
  "fact_reranker": null,
  "fact_ce_skip": false,
  "keyword_mode": "llm",
  "keyword_llm_provider": null,
  "keyword_llm_model": null,
  "extract_llm_provider": null,
  "extract_llm_model": null,
  "fact_protect_bm25": false,
  "coverage_protect_first": 0,
  "topic_entity_admit": false,
  "topic_ce_protect": false,
  "topic_trunc_protect": false,
  "topic_trunc_protect_max": 4,
  "topic_materialize": false,
  "topic_materialize_max": 4,
  "topic_materialize_content": false,
  "topic_materialize_types": "",
  "intent_rerank": false,
  "l2_bm25_union": true,
  "l2_bm25_mix_top_k": 30,
  "l2_bm25_mode": "fact_replace",
  "l2_fact_bm25_pool": "mix",
  "mix_intent_weights": false,
  "intent_factual_bias": false,
  "answer_prompt": "default",
  "answer_specific_types": "",
  "structure_induce": "off",
  "min_chunk_budget_ratio": 0.4,
  "query_arm_concurrency": 16,
  "adaptive_chunking": false,
  "chunk_token_size": 1200,
  "chunk_overlap_token_size": 100,
  "extract_max_entities": 40,
  "extract_max_records": 100,
  "extract_caps_selection": "fifo",
  "ingest_max_chars": null,
  "eq_query_concurrency": 4,
  "lr_query_concurrency_effective": 1,
  "fairness_note": "Matched top-k=30 budgets; LR rerank off (no model); L2 Evidence Recall + Context Relevancy required for valid smoke+; EQ Mix arm gate off (LR-like always-on local+global+naive) unless EDGEQUAKE_MIX_ARM_GATE=true on the server; Mix fusion default round_robin (SPEC-086 E2-occ; rrf is labeled ablation); Acc PATH_PRUNE=0 (022 P0; soft path only with CE+protect); Phase-1 EDGEQUAKE_MIX_RELEVANCY_PRUNE Acc default off; fair Acc ingest: adaptive_chunking off + chunk_token_size=1200 (LightRAG CHUNK_SIZE parity) unless explicitly ablated; fair Acc extract: 40/100 + EDGEQUAKE_EXTRACT_CAPS_SELECTION=fifo (SPEC-117; product default may be relation_aware); smoke-fast Acc may set BENCH001_INGEST_MAX_CHARS for fast force-ingest (full corpus = 0)",
  "eq_query_mode": "mix",
  "lr_query_mode": "mix",
  "chunk_size": 1200,
  "query_concurrency": 4,
  "eval_concurrency": 24,
  "judge": "generation_eval"
}
```

## Progression

- Ladder ledger: `specs/001-benchmark/e2e/artifacts/PROGRESS.md`
- This run archives under `specs/001-benchmark/e2e/artifacts/history/`
