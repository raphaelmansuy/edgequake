//! SPEC-103 — LightRAG-parity LLM response cache contracts.
//!
//! Normative cases from `specs/103-llm-cache/04-e2e-test-matrix.md`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use async_trait::async_trait;
use edgequake_llm::MockProvider;
use edgequake_query::cache::{
    hash_query_prompt, llm_cache_storage_key, resolve_llm_cache_flags, KvLlmResponseCache,
    LlmCacheType, LlmResponseCache, MemoryLlmResponseCache, TieredLlmResponseCache,
};
use edgequake_query::engine::QueryRequest;
use edgequake_query::keywords::{
    CachedKeywordExtractor, ExtractedKeywords, KeywordCache, KeywordExtractor, Keywords,
    QueryIntent, TieredKeywordCache,
};
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

struct CountingKeywordExtractor {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl KeywordExtractor for CountingKeywordExtractor {
    async fn extract(&self, query: &str) -> edgequake_query::Result<Keywords> {
        Ok(self.extract_extended(query).await?.to_simple())
    }

    async fn extract_extended(&self, query: &str) -> edgequake_query::Result<ExtractedKeywords> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExtractedKeywords::new(
            vec!["HIGH".into()],
            vec![query.to_ascii_uppercase()],
            QueryIntent::Factual,
        ))
    }
}

#[tokio::test]
async fn spec103_tiered_memory_then_postgres() {
    // MemoryKV stands in for public.llm_cache (same KV upsert/get contract).
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("spec103"));
    kv.initialize().await.unwrap();
    let tiered = TieredLlmResponseCache::with_kv(Arc::clone(&kv));
    let key = llm_cache_storage_key("mix", LlmCacheType::Query, &hash_query_prompt("p"));

    tiered
        .set_return(&key, LlmCacheType::Query, "cached-ans", Some("p"))
        .await;
    assert_eq!(tiered.get_return(&key).await.as_deref(), Some("cached-ans"));

    tiered.clear_l1();
    assert_eq!(
        tiered.get_return(&key).await.as_deref(),
        Some("cached-ans"),
        "L1 clear must still hit L2"
    );

    // Fresh tiered instance over same KV must also hit.
    let tiered2 = TieredLlmResponseCache::new(
        MemoryLlmResponseCache::with_defaults(),
        KvLlmResponseCache::new(kv),
    );
    assert_eq!(
        tiered2.get_return(&key).await.as_deref(),
        Some("cached-ans")
    );
}

#[tokio::test]
async fn spec103_keyword_hit_skips_llm() {
    let _g = ENV_LOCK.lock().await;
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
    std::env::remove_var("EDGEQUAKE_KEYWORD_CACHE");

    let calls = Arc::new(AtomicUsize::new(0));
    let inner: Arc<dyn KeywordExtractor> = Arc::new(CountingKeywordExtractor {
        calls: Arc::clone(&calls),
    });
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("spec103-kw"));
    kv.initialize().await.unwrap();
    let durable: Arc<dyn LlmResponseCache> =
        Arc::new(TieredLlmResponseCache::with_kv(Arc::clone(&kv)));
    let cache: Arc<dyn KeywordCache> =
        Arc::new(TieredKeywordCache::with_durable(100, Arc::clone(&durable)));
    let hit_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let extractor = CachedKeywordExtractor::with_model(
        inner,
        cache,
        Duration::from_secs(3600),
        "default",
        "mock-model",
        Arc::clone(&hit_flag),
    );

    let q = "What is BRCA1?";
    let _ = extractor.extract_extended(q).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!hit_flag.load(Ordering::Relaxed));

    let _ = extractor.extract_extended(q).await.unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "second extract must skip inner"
    );
    assert!(extractor.last_cache_hit());

    // New extractor + fresh L1; same KV L2 must serve without inner call.
    let durable2: Arc<dyn LlmResponseCache> = Arc::new(TieredLlmResponseCache::with_kv(kv));
    let cache2: Arc<dyn KeywordCache> = Arc::new(TieredKeywordCache::with_durable(100, durable2));
    let extractor2 = CachedKeywordExtractor::with_model(
        Arc::new(CountingKeywordExtractor {
            calls: Arc::clone(&calls),
        }),
        cache2,
        Duration::from_secs(3600),
        "default",
        "mock-model",
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );
    let before = calls.load(Ordering::SeqCst);
    let _ = extractor2.extract_extended(q).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), before);
    assert!(extractor2.last_cache_hit());

    std::env::remove_var("EDGEQUAKE_LLM_CACHE");
}

#[tokio::test]
async fn spec103_query_answer_hit_skips_generate() {
    let _g = ENV_LOCK.lock().await;
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
    std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");

    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("spec103", dim));
    let graph = Arc::new(MemoryGraphStorage::new("spec103"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    vector
        .upsert(&[(
            "chunk_a".to_string(),
            vec![1.0_f32; dim],
            serde_json::json!({
                "type": "chunk",
                "content": "BRCA1 is a tumor suppressor gene.",
                "document_id": "doc-a",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(MockProvider::default());
    mock.add_response("ANSWER_FROM_LLM_V1").await;
    mock.add_response("ANSWER_SHOULD_NOT_APPEAR").await;

    // QueryEngine::new wires CachedKeywordExtractor (not mock keywords).
    let engine = QueryEngine::new(
        QueryEngineConfig {
            mix_local_weight: 0.0,
            mix_global_weight: 0.0,
            mix_naive_weight: 1.0,
            ..Default::default()
        },
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
    .with_answer_cache();

    let mut req = QueryRequest::new("What is BRCA1?");
    req.mode = Some(QueryMode::Naive);

    let r1 = engine.query(req.clone()).await.expect("first query");
    assert!(
        !r1.context.is_empty(),
        "fixture must yield non-empty context for answer cache"
    );
    assert!(!r1.stats.answer_cache_hit);
    assert!(!r1.stats.keyword_cache_hit);
    assert_eq!(r1.answer, "ANSWER_FROM_LLM_V1");

    let r2 = engine.query(req).await.expect("second query");
    assert!(
        r2.stats.answer_cache_hit,
        "second identical Mix/Naive prompt must hit answer cache"
    );
    // WHY `keyword_cache_hit` is NOT asserted here: `QueryMode::Naive` runs the normal
    // retrieval pipeline, which goes straight from embedding to the dense+BM25 legs and never
    // calls the keyword extractor (`normal_pipeline::run_normal` → `retrieve_normal_context`).
    // A naive query therefore cannot report a keyword cache hit, and asserting one made this
    // test fail for a reason unrelated to the answer cache it exists to cover. The keyword
    // cache has its own test (`spec103_keyword_hit_skips_llm`).
    assert_eq!(r2.answer, "ANSWER_FROM_LLM_V1");

    std::env::remove_var("EDGEQUAKE_LLM_CACHE");
}

#[tokio::test]
async fn spec103_master_off_disables_both() {
    let _g = ENV_LOCK.lock().await;
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "0");
    std::env::remove_var("EDGEQUAKE_KEYWORD_CACHE");
    std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");

    let f = resolve_llm_cache_flags();
    assert!(!f.master && !f.keywords && !f.answer);

    let calls = Arc::new(AtomicUsize::new(0));
    let inner: Arc<dyn KeywordExtractor> = Arc::new(CountingKeywordExtractor {
        calls: Arc::clone(&calls),
    });
    let cache: Arc<dyn KeywordCache> = Arc::new(TieredKeywordCache::memory_only(100));
    let extractor = CachedKeywordExtractor::with_model(
        inner,
        cache,
        Duration::from_secs(60),
        "default",
        "m",
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );
    let _ = extractor.extract_extended("q").await.unwrap();
    let _ = extractor.extract_extended("q").await.unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "master off must disable keyword cache"
    );

    std::env::remove_var("EDGEQUAKE_LLM_CACHE");
}

#[tokio::test]
async fn spec103_persist_across_engine_rebuild() {
    let _g = ENV_LOCK.lock().await;
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
    std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");

    let dim = 1536;
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("spec103-persist"));
    kv.initialize().await.unwrap();

    let vector = Arc::new(MemoryVectorStorage::new("spec103-persist", dim));
    let graph = Arc::new(MemoryGraphStorage::new("spec103-persist"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();
    vector
        .upsert(&[(
            "chunk_p".to_string(),
            vec![1.0_f32; dim],
            serde_json::json!({
                "type": "chunk",
                "content": "persistent cache content",
                "document_id": "doc-p",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(MockProvider::default());
    mock.add_response("PERSISTED_ANSWER").await;
    mock.add_response("SHOULD_NOT_USE").await;

    let engine1 = QueryEngine::with_mock_keywords(
        QueryEngineConfig {
            mix_local_weight: 0.0,
            mix_global_weight: 0.0,
            mix_naive_weight: 1.0,
            ..Default::default()
        },
        Arc::clone(&vector) as Arc<dyn VectorStorage>,
        Arc::clone(&graph) as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
    .with_answer_cache()
    .with_kv_storage(Arc::clone(&kv));

    let mut req = QueryRequest::new("persist me");
    req.mode = Some(QueryMode::Naive);
    let r1 = engine1.query(req.clone()).await.expect("engine1");
    assert!(!r1.stats.answer_cache_hit);
    assert_eq!(r1.answer, "PERSISTED_ANSWER");

    // Rebuild engine — L1 empty, L2 (MemoryKV) must serve.
    let engine2 = QueryEngine::with_mock_keywords(
        QueryEngineConfig {
            mix_local_weight: 0.0,
            mix_global_weight: 0.0,
            mix_naive_weight: 1.0,
            ..Default::default()
        },
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
    .with_answer_cache()
    .with_kv_storage(kv);

    let r2 = engine2.query(req).await.expect("engine2");
    assert!(
        r2.stats.answer_cache_hit,
        "rebuild with same namespace KV must hit L2"
    );
    assert_eq!(r2.answer, "PERSISTED_ANSWER");

    std::env::remove_var("EDGEQUAKE_LLM_CACHE");
}

#[tokio::test]
async fn spec103_vision_or_empty_context_bypass() {
    let _g = ENV_LOCK.lock().await;
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "1");
    std::env::remove_var("EDGEQUAKE_QUERY_ANSWER_CACHE");

    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("spec103-empty", dim));
    let graph = Arc::new(MemoryGraphStorage::new("spec103-empty"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();
    // No chunks → empty context → must not poison cache with empty/apology.

    let mock = Arc::new(MockProvider::default());
    mock.add_response("EMPTY_CTX_A").await;
    mock.add_response("EMPTY_CTX_B").await;

    let engine = QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
    .with_answer_cache();

    let mut req = QueryRequest::new("no context query");
    req.mode = Some(QueryMode::Naive);

    let r1 = engine.query(req.clone()).await.expect("q1");
    assert!(r1.context.is_empty());
    assert!(!r1.stats.answer_cache_hit);

    let r2 = engine.query(req).await.expect("q2");
    assert!(
        !r2.stats.answer_cache_hit,
        "empty context must not answer-cache"
    );

    std::env::remove_var("EDGEQUAKE_LLM_CACHE");
}
