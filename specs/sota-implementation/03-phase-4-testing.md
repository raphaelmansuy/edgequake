# Phase 4: Testing Strategy

## Objective

Comprehensive test coverage for all SOTA features: unit, integration, and E2E.

## Duration: 4-6 hours

---

## Unit Tests

### 1. GleaningExtractor Tests

**File**: `edgequake/crates/edgequake-pipeline/src/extractor.rs` (append to existing tests)

```rust
#[cfg(test)]
mod gleaning_tests {
    use super::*;
    use crate::test_utils::mock_llm_provider;

    #[tokio::test]
    async fn test_gleaning_single_pass() {
        let mock_llm = mock_llm_provider(vec![
            // First extraction
            r#"[{"name":"OPENAI","type":"ORG","description":"AI company"}]"#,
            // Gleaning pass
            r#"[{"name":"SAM_ALTMAN","type":"PERSON","description":"CEO of OpenAI"}]"#,
        ]);

        let base = Arc::new(LLMExtractor::new(mock_llm.clone()));
        let gleaning = GleaningExtractor::new(mock_llm, base)
            .with_max_gleaning(1);

        let result = gleaning.extract("OpenAI was founded by Sam Altman.").await?;

        // Should have entities from both passes
        assert_eq!(result.entities.len(), 2);
        assert!(result.entities.iter().any(|e| e.name == "OPENAI"));
        assert!(result.entities.iter().any(|e| e.name == "SAM_ALTMAN"));

        // Metadata should track iterations
        assert_eq!(result.metadata.get("gleaning_iterations"), Some(&1));
    }

    #[tokio::test]
    async fn test_gleaning_merge_duplicates() {
        let mock_llm = mock_llm_provider(vec![
            // First extraction
            r#"[{"name":"OPENAI","type":"ORG","description":"Founded 2015"}]"#,
            // Gleaning finds same entity with different description
            r#"[{"name":"OPENAI","type":"ORG","description":"Created ChatGPT"}]"#,
        ]);

        let base = Arc::new(LLMExtractor::new(mock_llm.clone()));
        let gleaning = GleaningExtractor::new(mock_llm, base)
            .with_max_gleaning(1);

        let result = gleaning.extract("OpenAI...").await?;

        // Should deduplicate to one entity
        assert_eq!(result.entities.len(), 1);

        // Should keep longer description
        let openai = &result.entities[0];
        assert!(openai.description.len() > 10);
    }

    #[tokio::test]
    async fn test_gleaning_stops_when_no_new_entities() {
        let mock_llm = mock_llm_provider(vec![
            // First extraction
            r#"[{"name":"OPENAI","type":"ORG","description":"AI company"}]"#,
            // Gleaning finds nothing new
            r#"[]"#,
        ]);

        let base = Arc::new(LLMExtractor::new(mock_llm.clone()));
        let gleaning = GleaningExtractor::new(mock_llm, base)
            .with_max_gleaning(3);

        let result = gleaning.extract("OpenAI...").await?;

        // Should stop after first empty gleaning
        assert_eq!(result.metadata.get("gleaning_iterations"), Some(&1));
    }

    #[tokio::test]
    async fn test_gleaning_respects_max_iterations() {
        let mock_llm = mock_llm_provider(vec![
            r#"[{"name":"ENTITY1","type":"ORG","description":"desc"}]"#,
            r#"[{"name":"ENTITY2","type":"ORG","description":"desc"}]"#,
            r#"[{"name":"ENTITY3","type":"ORG","description":"desc"}]"#,
            r#"[{"name":"ENTITY4","type":"ORG","description":"desc"}]"#,
        ]);

        let base = Arc::new(LLMExtractor::new(mock_llm.clone()));
        let gleaning = GleaningExtractor::new(mock_llm, base)
            .with_max_gleaning(2);

        let result = gleaning.extract("...").await?;

        // Should stop at max_gleaning
        assert_eq!(result.metadata.get("gleaning_iterations"), Some(&2));
        assert_eq!(result.entities.len(), 3); // base + 2 gleaning
    }
}
```

### 2. LLMSummarizer Tests

**File**: `edgequake/crates/edgequake-pipeline/src/summarizer.rs` (append to existing tests)

```rust
#[cfg(test)]
mod summarizer_tests {
    use super::*;

    #[tokio::test]
    async fn test_merge_entity_descriptions_short() {
        let mock_llm = mock_llm_provider(vec![]);
        let summarizer = LLMSummarizer::new(mock_llm, SummarizerConfig::default());

        let descriptions = vec![
            "Founded in 2015.".to_string(),
            "Created ChatGPT.".to_string(),
        ];

        let result = summarizer.merge_entity_descriptions("OPENAI", &descriptions).await?;

        // Short descriptions should be concatenated, not LLM-summarized
        assert!(result.contains("2015") || result.contains("ChatGPT"));
    }

    #[tokio::test]
    async fn test_merge_entity_descriptions_long() {
        let mock_llm = mock_llm_provider(vec![
            "OpenAI is an AI research company founded in 2015 that created ChatGPT.",
        ]);

        let summarizer = LLMSummarizer::new(mock_llm, SummarizerConfig {
            force_llm_summary_threshold: 50, // Low threshold
            ..Default::default()
        });

        let descriptions = vec![
            "A".repeat(100),
            "B".repeat(100),
        ];

        let result = summarizer.merge_entity_descriptions("OPENAI", &descriptions).await?;

        // Long descriptions should be LLM-summarized
        assert!(result.len() < 200); // Should be shorter than concatenation
    }

    #[tokio::test]
    async fn test_map_reduce_summarization() {
        let mock_llm = mock_llm_provider(vec![
            "Summary of chunk 1",
            "Summary of chunk 2",
            "Final merged summary",
        ]);

        let summarizer = LLMSummarizer::new(mock_llm, SummarizerConfig {
            chunk_size: 50,
            ..Default::default()
        });

        // Create many descriptions that exceed chunk size
        let descriptions: Vec<_> = (0..10)
            .map(|i| format!("Description {} with some content.", i))
            .collect();

        let result = summarizer.merge_entity_descriptions("ENTITY", &descriptions).await?;

        assert!(!result.is_empty());
    }
}
```

### 3. Reranker Tests

**File**: `edgequake/crates/edgequake-llm/src/reranker.rs` (append to existing tests)

```rust
#[cfg(test)]
mod reranker_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_reranker_scores() {
        let reranker = MockReranker::new();

        let documents = vec![
            "OpenAI is an AI company.".to_string(),
            "Weather is nice today.".to_string(),
            "GPT-4 was released in 2023.".to_string(),
        ];

        let results = reranker.rerank("What is OpenAI?", &documents, None).await?;

        // Should return all documents with scores
        assert_eq!(results.len(), 3);

        // All scores should be between 0 and 1
        for r in &results {
            assert!(r.relevance_score >= 0.0 && r.relevance_score <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_reranker_respects_top_k() {
        let reranker = MockReranker::new();

        let documents: Vec<_> = (0..20).map(|i| format!("Document {}", i)).collect();

        let results = reranker.rerank("query", &documents, Some(5)).await?;

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_reranker_empty_input() {
        let reranker = MockReranker::new();

        let results = reranker.rerank("query", &[], None).await?;

        assert!(results.is_empty());
    }
}
```

### 4. Degree Ranking Tests

**File**: `edgequake/crates/edgequake-query/src/sota_engine.rs` (append to existing tests)

```rust
#[cfg(test)]
mod degree_tests {
    use super::*;

    #[tokio::test]
    async fn test_rank_by_degree() {
        let graph = create_test_graph_with_degrees().await;
        let engine = SOTAQueryEngine::new(graph, ...);

        let entities = vec![
            RetrievedEntity { id: "OPENAI".into(), degree: 0, .. },
            RetrievedEntity { id: "SAM_ALTMAN".into(), degree: 0, .. },
            RetrievedEntity { id: "CHATGPT".into(), degree: 0, .. },
        ];

        let ranked = engine.rank_by_degree(entities).await?;

        // Should be sorted by degree descending
        for i in 1..ranked.len() {
            assert!(ranked[i-1].degree >= ranked[i].degree);
        }
    }

    #[tokio::test]
    async fn test_node_degrees_batch() {
        let graph = create_test_graph().await;

        // Add some edges
        graph.add_edge("OPENAI", "CHATGPT", "CREATED").await?;
        graph.add_edge("OPENAI", "GPT4", "CREATED").await?;
        graph.add_edge("SAM_ALTMAN", "OPENAI", "LEADS").await?;

        let degrees = graph.node_degrees_batch(&["OPENAI", "SAM_ALTMAN", "CHATGPT"]).await?;

        // OPENAI should have highest degree (3 connections)
        assert!(degrees.get("OPENAI").copied().unwrap_or(0) >= 3);
        // SAM_ALTMAN has 1 connection
        assert_eq!(degrees.get("SAM_ALTMAN").copied().unwrap_or(0), 1);
        // CHATGPT has 1 connection
        assert_eq!(degrees.get("CHATGPT").copied().unwrap_or(0), 1);
    }
}
```

---

## Integration Tests

### 1. Pipeline Integration Tests

**File**: `edgequake/crates/edgequake-core/tests/pipeline_integration.rs`

```rust
use edgequake_core::Pipeline;
use edgequake_storage::MemoryGraphStorage;

#[tokio::test]
async fn test_full_pipeline_with_gleaning() {
    let storage = Arc::new(MemoryGraphStorage::new());
    let pipeline = Pipeline::builder()
        .with_storage(storage.clone())
        .with_gleaning(true, 1)
        .with_llm_summarization(true)
        .build()
        .await?;

    // Process a document
    let result = pipeline.process(
        "doc1",
        "Sarah Chen founded OpenAI with Sam Altman in San Francisco. \
         OpenAI later created ChatGPT, which became widely popular. \
         Sam Altman is now the CEO of OpenAI."
    ).await?;

    // Should have extracted entities with gleaning
    assert!(result.stats.entities_count >= 4); // SARAH_CHEN, OPENAI, SAM_ALTMAN, SAN_FRANCISCO, CHATGPT
    assert!(result.stats.gleaning_iterations >= 1);

    // Check deduplication worked
    let nodes = storage.get_all_nodes().await?;
    let openai_nodes: Vec<_> = nodes.iter().filter(|n| n.id == "OPENAI").collect();
    assert_eq!(openai_nodes.len(), 1); // Should be deduplicated

    // Check description was merged
    let openai = openai_nodes[0];
    let desc = openai.properties.get("description").unwrap().as_str().unwrap();
    // Should contain info from both mentions
    assert!(desc.contains("founded") || desc.contains("CEO") || desc.contains("ChatGPT"));
}

#[tokio::test]
async fn test_pipeline_without_gleaning() {
    let storage = Arc::new(MemoryGraphStorage::new());
    let pipeline = Pipeline::builder()
        .with_storage(storage.clone())
        .with_gleaning(false, 0)
        .build()
        .await?;

    let result = pipeline.process("doc1", "OpenAI created ChatGPT.").await?;

    // Should work without gleaning
    assert!(result.stats.entities_count >= 1);
    assert_eq!(result.stats.gleaning_iterations, 0);
}
```

### 2. Query Integration Tests

**File**: `edgequake/crates/edgequake-core/tests/query_integration.rs`

```rust
use edgequake_query::SOTAQueryEngine;

#[tokio::test]
async fn test_query_with_reranking() {
    // Setup: Create graph with test data
    let (graph, vector_store, llm) = setup_test_stores().await;

    // Add test data
    add_test_entities(&graph, &vector_store).await;

    let reranker = Arc::new(MockReranker::new());
    let engine = SOTAQueryEngine::builder()
        .with_graph(graph)
        .with_vectors(vector_store)
        .with_llm(llm)
        .with_reranker(reranker)
        .with_config(SOTAQueryConfig {
            enable_rerank: true,
            min_rerank_score: 0.3,
            ..Default::default()
        })
        .build();

    let response = engine.query("What is OpenAI?").await?;

    assert!(!response.answer.is_empty());
    assert!(response.stats.chunks_count > 0);
}

#[tokio::test]
async fn test_query_adaptive_mode() {
    let (graph, vector_store, llm) = setup_test_stores().await;
    add_test_entities(&graph, &vector_store).await;

    let engine = SOTAQueryEngine::builder()
        .with_graph(graph)
        .with_vectors(vector_store)
        .with_llm(llm)
        .with_config(SOTAQueryConfig {
            mode: QueryMode::Adaptive,
            enable_keywords: true,
            ..Default::default()
        })
        .build();

    // Local question
    let local_response = engine.query("What did Sarah Chen do?").await?;
    assert!(local_response.mode_used == QueryMode::Local ||
            local_response.mode_used == QueryMode::Hybrid);

    // Global question
    let global_response = engine.query("What are the main themes in the documents?").await?;
    assert!(global_response.mode_used == QueryMode::Global ||
            global_response.mode_used == QueryMode::Hybrid);
}

#[tokio::test]
async fn test_query_degree_ranking() {
    let (graph, vector_store, llm) = setup_test_stores().await;

    // Create graph with varying degrees
    graph.add_node("HUB_ENTITY", "ORG", "Very connected").await?;
    for i in 0..10 {
        let leaf = format!("LEAF_{}", i);
        graph.add_node(&leaf, "THING", "Leaf entity").await?;
        graph.add_edge("HUB_ENTITY", &leaf, "CONNECTS_TO").await?;
    }
    graph.add_node("ISOLATED_ENTITY", "ORG", "No connections").await?;

    let engine = SOTAQueryEngine::builder()
        .with_graph(graph)
        .with_vectors(vector_store)
        .with_llm(llm)
        .with_config(SOTAQueryConfig {
            enable_degree_ranking: true,
            ..Default::default()
        })
        .build();

    let response = engine.query("Tell me about the entities.").await?;

    // HUB_ENTITY should be mentioned prominently (higher degree = more important)
    assert!(response.context.entities.iter()
        .position(|e| e.id == "HUB_ENTITY")
        .unwrap_or(usize::MAX) < 3);
}
```

---

## E2E Tests

### 1. API E2E Tests

**File**: `edgequake/crates/edgequake-api/tests/e2e_sota.rs`

```rust
use axum_test::TestServer;
use serde_json::json;

async fn setup_server() -> TestServer {
    let app = create_app_with_mock_llm().await;
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_e2e_ingest_with_gleaning() {
    let server = setup_server().await;

    let response = server
        .post("/api/documents")
        .json(&json!({
            "content": "Sarah Chen is the founder of TechCorp. She lives in San Francisco. \
                       TechCorp develops AI software. Sam works with Sarah at TechCorp.",
            "enable_gleaning": true,
            "max_gleaning": 1,
            "use_llm_summarization": true
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["stats"]["entities_count"].as_u64().unwrap() >= 3);
    assert!(body["stats"]["gleaning_iterations"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_e2e_query_with_all_features() {
    let server = setup_server().await;

    // First ingest some data
    server
        .post("/api/documents")
        .json(&json!({
            "content": "OpenAI is an AI research company. It created ChatGPT. \
                       Sam Altman is the CEO of OpenAI.",
            "enable_gleaning": true
        }))
        .await
        .assert_status_ok();

    // Now query with all SOTA features
    let response = server
        .post("/api/query")
        .json(&json!({
            "query": "What is OpenAI and who leads it?",
            "mode": "adaptive",
            "enable_rerank": true,
            "rerank_model": "jina",
            "enable_keywords": true,
            "enable_degree_ranking": true,
            "include_sources": true
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(!body["answer"].as_str().unwrap().is_empty());
    assert!(body["stats"]["entities_count"].as_u64().unwrap() > 0);

    // Should have sources
    let sources = body["sources"].as_array().unwrap();
    assert!(!sources.is_empty());
}

#[tokio::test]
async fn test_e2e_config_endpoint() {
    let server = setup_server().await;

    let response = server.get("/api/config").await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(!body["llm_model"].as_str().unwrap().is_empty());
    assert!(body["default_query_config"]["token_budget"].as_u64().unwrap() > 0);
    assert!(body["default_ingest_config"]["enable_gleaning"].as_bool().unwrap());
}

#[tokio::test]
async fn test_e2e_query_modes() {
    let server = setup_server().await;

    // Ingest test data
    server
        .post("/api/documents")
        .json(&json!({
            "content": "Document about AI companies and their products."
        }))
        .await
        .assert_status_ok();

    // Test each mode
    for mode in ["local", "global", "hybrid", "adaptive"] {
        let response = server
            .post("/api/query")
            .json(&json!({
                "query": "What are the AI companies?",
                "mode": mode
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(!body["answer"].as_str().unwrap().is_empty());
    }
}
```

### 2. UI E2E Tests (Playwright)

**File**: `edgequake_webui/e2e/sota-features.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("SOTA Features", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("Query page shows advanced settings", async ({ page }) => {
    await page.goto("/query");

    // Advanced settings should be collapsed by default
    const advancedButton = page.getByRole("button", {
      name: /advanced settings/i,
    });
    await expect(advancedButton).toBeVisible();

    // Expand
    await advancedButton.click();

    // Should show reranking options
    await expect(page.getByLabel(/enable reranking/i)).toBeVisible();
    await expect(page.getByLabel(/model/i)).toBeVisible();

    // Should show entity ranking options
    await expect(page.getByLabel(/enable keyword extraction/i)).toBeVisible();
    await expect(page.getByLabel(/enable degree ranking/i)).toBeVisible();

    // Should show limits
    await expect(page.getByLabel(/max entities/i)).toBeVisible();
    await expect(page.getByLabel(/token budget/i)).toBeVisible();
  });

  test("Ingest page shows gleaning options", async ({ page }) => {
    await page.goto("/documents");

    // Should show gleaning toggle
    await expect(page.getByLabel(/multi-pass gleaning/i)).toBeVisible();

    // Should show summarization toggle
    await expect(page.getByLabel(/llm description merging/i)).toBeVisible();

    // Should show chunking options
    await expect(page.getByLabel(/chunk size/i)).toBeVisible();
    await expect(page.getByLabel(/chunk overlap/i)).toBeVisible();
  });

  test("Query with reranking shows rerank scores", async ({ page }) => {
    await page.goto("/query");

    // Enable reranking
    await page.getByRole("button", { name: /advanced settings/i }).click();
    await page.getByLabel(/enable reranking/i).check();

    // Submit query
    await page.getByPlaceholder(/ask a question/i).fill("What is OpenAI?");
    await page.getByRole("button", { name: /submit|search/i }).click();

    // Wait for response
    await page.waitForSelector('[data-testid="query-answer"]');

    // Should show sources with rerank scores
    const sources = page.locator('[data-testid="source-item"]');
    await expect(sources.first()).toBeVisible();

    // Should have rerank score badge
    await expect(page.getByText(/R:/)).toBeVisible();
  });

  test("Query stats display correctly", async ({ page }) => {
    await page.goto("/query");

    // Submit query
    await page.getByPlaceholder(/ask a question/i).fill("What is AI?");
    await page.getByRole("button", { name: /submit|search/i }).click();

    // Wait for response
    await page.waitForSelector('[data-testid="query-stats"]');

    // Should show mode
    await expect(page.getByText(/mode/i)).toBeVisible();

    // Should show latency
    await expect(page.getByText(/latency/i)).toBeVisible();

    // Should show entity count
    await expect(page.getByText(/entities/i)).toBeVisible();
  });

  test("Ingest with gleaning shows stats", async ({ page }) => {
    await page.goto("/documents");

    // Enable gleaning
    await page.getByLabel(/multi-pass gleaning/i).check();

    // Upload a file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: "test.txt",
      mimeType: "text/plain",
      buffer: Buffer.from(
        "Sarah Chen founded OpenAI with Sam Altman in San Francisco."
      ),
    });

    // Submit
    await page.getByRole("button", { name: /upload|ingest/i }).click();

    // Wait for result
    await page.waitForSelector('[data-testid="ingest-result"]');

    // Should show gleaning iterations
    await expect(page.getByText(/gleaning iterations/i)).toBeVisible();

    // Should show entities extracted
    await expect(page.getByText(/entities/i)).toBeVisible();
  });
});
```

---

## Performance Tests

**File**: `edgequake/crates/edgequake-core/benches/sota_bench.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

fn bench_gleaning(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("gleaning_1_pass", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pipeline = create_bench_pipeline(1).await;
                pipeline.process("doc", BENCH_DOCUMENT).await.unwrap()
            })
        })
    });

    c.bench_function("gleaning_2_passes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pipeline = create_bench_pipeline(2).await;
                pipeline.process("doc", BENCH_DOCUMENT).await.unwrap()
            })
        })
    });
}

fn bench_reranking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("query_without_rerank", |b| {
        b.iter(|| {
            rt.block_on(async {
                let engine = create_bench_engine(false).await;
                engine.query("What is OpenAI?").await.unwrap()
            })
        })
    });

    c.bench_function("query_with_rerank", |b| {
        b.iter(|| {
            rt.block_on(async {
                let engine = create_bench_engine(true).await;
                engine.query("What is OpenAI?").await.unwrap()
            })
        })
    });
}

fn bench_degree_ranking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("degree_ranking_100_entities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (graph, _) = create_bench_graph(100).await;
                let engine = create_engine_with_graph(graph).await;
                engine.query("entities").await.unwrap()
            })
        })
    });
}

criterion_group!(benches, bench_gleaning, bench_reranking, bench_degree_ranking);
criterion_main!(benches);
```

---

## Test Data Fixtures

**File**: `edgequake/crates/edgequake-core/tests/fixtures.rs`

```rust
pub const TEST_DOCUMENT_1: &str = r#"
Sarah Chen is the founder and CEO of TechCorp, a leading AI company based in San Francisco.
Before founding TechCorp, Sarah worked at Google where she led the AI research team.
TechCorp has developed several groundbreaking AI products, including SmartAssist and AIWriter.
The company has partnerships with Microsoft, Amazon, and several Fortune 500 companies.
Sarah holds a PhD in Computer Science from Stanford University.
"#;

pub const TEST_DOCUMENT_2: &str = r#"
OpenAI was founded in 2015 by Sam Altman, Elon Musk, and others.
The company is headquartered in San Francisco and focuses on AI safety research.
OpenAI created ChatGPT, which became the fastest-growing consumer application in history.
GPT-4, their latest model, demonstrates remarkable reasoning capabilities.
Sam Altman serves as the CEO of OpenAI.
"#;

pub struct TestFixtures {
    pub documents: Vec<&'static str>,
    pub expected_entities: Vec<&'static str>,
    pub expected_relationships: Vec<(&'static str, &'static str, &'static str)>,
}

impl TestFixtures {
    pub fn small() -> Self {
        Self {
            documents: vec![TEST_DOCUMENT_1],
            expected_entities: vec!["SARAH_CHEN", "TECHCORP", "SAN_FRANCISCO", "GOOGLE"],
            expected_relationships: vec![
                ("SARAH_CHEN", "TECHCORP", "FOUNDER_OF"),
                ("TECHCORP", "SAN_FRANCISCO", "BASED_IN"),
            ],
        }
    }

    pub fn medium() -> Self {
        Self {
            documents: vec![TEST_DOCUMENT_1, TEST_DOCUMENT_2],
            expected_entities: vec![
                "SARAH_CHEN", "TECHCORP", "SAN_FRANCISCO", "GOOGLE",
                "OPENAI", "SAM_ALTMAN", "CHATGPT", "GPT_4",
            ],
            expected_relationships: vec![
                ("SARAH_CHEN", "TECHCORP", "FOUNDER_OF"),
                ("SAM_ALTMAN", "OPENAI", "CEO_OF"),
                ("OPENAI", "CHATGPT", "CREATED"),
            ],
        }
    }
}
```

---

## Verification Checklist

- [ ] All unit tests pass: `cargo test --lib`
- [ ] All integration tests pass: `cargo test --test`
- [ ] All E2E API tests pass: `cargo test --package edgequake-api --test e2e`
- [ ] All UI E2E tests pass: `pnpm exec playwright test`
- [ ] Performance benchmarks complete: `cargo bench`
- [ ] Code coverage > 80%: `cargo llvm-cov`
- [ ] No regressions in existing tests

---

## Cross-References

- **Previous Phase**: [02-phase-3-ui-integration.md](02-phase-3-ui-integration.md)
- **Next Phase**: [04-phase-5-validation.md](04-phase-5-validation.md)
- **Current State**: [00-current-state-analysis.md](00-current-state-analysis.md)
