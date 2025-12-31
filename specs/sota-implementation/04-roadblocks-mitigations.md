# Roadblocks and Mitigations

## Objective

Predict potential issues and define proactive solutions.

---

## Roadblock 1: LLM API Rate Limits

### Risk Level: HIGH

### Description

When enabling gleaning (multiple LLM passes) and LLM-based summarization, the number of API calls increases significantly. This can hit rate limits, especially with OpenAI's API.

### Symptoms

- `429 Too Many Requests` errors
- Slow processing due to backpressure
- Incomplete entity extraction

### Mitigation Strategies

1. **Implement Exponential Backoff**

   ```rust
   // Already exists in edgequake-llm/src/provider.rs
   async fn call_with_retry(&self, request: &Request) -> Result<Response> {
       let mut delay = Duration::from_millis(100);
       for attempt in 0..self.max_retries {
           match self.call(request).await {
               Ok(response) => return Ok(response),
               Err(e) if e.is_rate_limit() => {
                   tokio::time::sleep(delay).await;
                   delay *= 2;
               }
               Err(e) => return Err(e),
           }
       }
       Err(Error::MaxRetriesExceeded)
   }
   ```

2. **Batch API Calls**

   - Use batch endpoints where available
   - Group entity extractions by document

3. **Configurable Concurrency**

   ```rust
   pub struct PipelineConfig {
       /// Maximum concurrent LLM calls
       pub max_concurrent_llm_calls: usize, // Default: 5
   }
   ```

4. **Smart Gleaning**
   - Only glean if initial extraction is below threshold
   - Skip gleaning for very short documents

---

## Roadblock 2: Memory Pressure with Large Documents

### Risk Level: MEDIUM

### Description

Large documents (>100KB) with multiple gleaning passes can consume significant memory, especially when storing intermediate results.

### Symptoms

- OOM errors on resource-constrained systems
- Slow processing due to swapping
- Incomplete document processing

### Mitigation Strategies

1. **Streaming Chunk Processing**

   ```rust
   async fn process_document_streaming(&self, doc: &Document) -> Result<Stats> {
       let mut stats = Stats::default();

       for chunk in doc.chunks() {
           let result = self.process_chunk(chunk).await?;
           stats.merge(result);

           // Commit to storage after each chunk
           self.storage.commit_batch().await?;
       }

       Ok(stats)
   }
   ```

2. **Lazy Description Merging**

   - Don't hold all descriptions in memory
   - Use streaming summarization

3. **Configurable Memory Limits**
   ```rust
   pub struct PipelineConfig {
       /// Maximum entities to hold in memory
       pub max_entities_in_memory: usize, // Default: 10000

       /// Flush threshold
       pub flush_threshold: usize, // Default: 1000
   }
   ```

---

## Roadblock 3: Reranker API Unavailability

### Risk Level: MEDIUM

### Description

External reranking services (Jina, Cohere) may be unavailable or slow, blocking query execution.

### Symptoms

- Query timeouts
- 5xx errors from reranker APIs
- Degraded query quality when fallback is used

### Mitigation Strategies

1. **Graceful Fallback**

   ```rust
   async fn rerank_with_fallback(
       &self,
       query: &str,
       chunks: Vec<Chunk>,
   ) -> Result<Vec<Chunk>> {
       match self.reranker.rerank(query, &chunks, self.config.rerank_top_k).await {
           Ok(reranked) => Ok(reranked),
           Err(e) => {
               tracing::warn!("Reranking failed, using similarity fallback: {}", e);
               // Fallback to vector similarity ranking
               Ok(self.rank_by_similarity(query, chunks).await)
           }
       }
   }
   ```

2. **Local Reranker Option**

   - Add support for local cross-encoder models
   - Use ONNX runtime for inference

3. **Caching Rerank Results**
   - Cache rerank scores for common query patterns
   - TTL-based invalidation

---

## Roadblock 4: Graph Storage Performance

### Risk Level: MEDIUM

### Description

`node_degrees_batch` can be slow for large graphs with PostgreSQL AGE, especially without proper indexing.

### Symptoms

- Slow query response times
- High database CPU usage
- Query timeouts

### Mitigation Strategies

1. **Add Materialized Degree Column**

   ```sql
   -- Add degree column to nodes table
   ALTER TABLE nodes ADD COLUMN degree INTEGER DEFAULT 0;

   -- Create index
   CREATE INDEX idx_nodes_degree ON nodes(degree DESC);

   -- Trigger to update on edge changes
   CREATE OR REPLACE FUNCTION update_node_degree()
   RETURNS TRIGGER AS $$
   BEGIN
       UPDATE nodes SET degree = (
           SELECT COUNT(*) FROM edges
           WHERE source_id = NEW.source_id OR target_id = NEW.source_id
       ) WHERE id = NEW.source_id;
       RETURN NEW;
   END;
   $$ LANGUAGE plpgsql;
   ```

2. **Batch Degree Queries**

   ```rust
   async fn node_degrees_batch(&self, ids: &[String]) -> Result<HashMap<String, usize>> {
       // Single query instead of N queries
       let query = "
           SELECT id, degree FROM nodes WHERE id = ANY($1)
       ";
       self.pool.query(query, &[&ids]).await
   }
   ```

3. **Periodic Degree Recalculation**
   - Background job to update degrees
   - Avoid real-time calculation

---

## Roadblock 5: UI State Management Complexity

### Risk Level: LOW

### Description

Advanced settings introduce complex state that needs to persist across navigation and be synchronized with server config.

### Symptoms

- Settings reset on navigation
- Inconsistent state between UI and server
- Poor UX with many form fields

### Mitigation Strategies

1. **Use React Query for Server State**

   ```tsx
   const { data: config } = useQuery({
     queryKey: ["config"],
     queryFn: fetchConfig,
   });

   // Initialize settings from server config
   const [settings, setSettings] = useState(() => ({
     ...defaultSettings,
     ...config?.defaults,
   }));
   ```

2. **Persist to localStorage**

   ```tsx
   useEffect(() => {
     localStorage.setItem("query-settings", JSON.stringify(settings));
   }, [settings]);
   ```

3. **Presets for Common Configurations**
   ```tsx
   const PRESETS = {
     fast: { enableRerank: false, maxGleaning: 0 },
     balanced: { enableRerank: true, maxGleaning: 1 },
     thorough: { enableRerank: true, maxGleaning: 2 },
   };
   ```

---

## Roadblock 6: Test Flakiness

### Risk Level: LOW

### Description

Tests involving LLM calls or timing-sensitive operations may be flaky.

### Symptoms

- Intermittent test failures
- CI pipeline instability
- Difficulty debugging

### Mitigation Strategies

1. **Deterministic Mocks**

   ```rust
   pub struct MockLLMProvider {
       responses: Vec<String>,
       current: AtomicUsize,
   }

   impl MockLLMProvider {
       pub fn new(responses: Vec<String>) -> Self {
           Self { responses, current: AtomicUsize::new(0) }
       }

       async fn call(&self, _request: &Request) -> Result<Response> {
           let idx = self.current.fetch_add(1, Ordering::SeqCst);
           Ok(Response::new(self.responses[idx % self.responses.len()].clone()))
       }
   }
   ```

2. **Retry Flaky Tests**

   ```toml
   # Cargo.toml
   [dev-dependencies]
   test-retry = "0.1"
   ```

3. **Increase Timeouts for Integration Tests**
   ```rust
   #[tokio::test(flavor = "multi_thread")]
   #[timeout(Duration::from_secs(30))]
   async fn test_full_pipeline() { ... }
   ```

---

## Roadblock 7: Breaking API Changes

### Risk Level: MEDIUM

### Description

Extending API schemas may break existing clients if not done carefully.

### Symptoms

- Client errors after deployment
- Version mismatches
- Frontend/backend desync

### Mitigation Strategies

1. **Backward-Compatible Defaults**

   ```rust
   #[derive(Deserialize)]
   pub struct QueryRequest {
       pub query: String,

       // All new fields have defaults
       #[serde(default)]
       pub enable_rerank: bool,

       #[serde(default = "default_mode")]
       pub mode: QueryMode,
   }
   ```

2. **API Versioning**

   ```rust
   // v1 routes (existing)
   .route("/api/v1/query", post(handlers::v1::query))

   // v2 routes (new with SOTA features)
   .route("/api/v2/query", post(handlers::v2::query))

   // Unversioned defaults to latest
   .route("/api/query", post(handlers::v2::query))
   ```

3. **Feature Flags**
   ```rust
   if request.features.contains("rerank") {
       // Use new reranking path
   } else {
       // Use legacy path
   }
   ```

---

## Roadblock 8: Cost Overruns

### Risk Level: MEDIUM

### Description

Gleaning and LLM summarization increase LLM API costs significantly.

### Symptoms

- Unexpectedly high bills
- User complaints about cost
- Need to disable features

### Mitigation Strategies

1. **Cost Tracking**

   ```rust
   pub struct IngestStats {
       pub tokens_used: usize,
       pub estimated_cost_usd: f64,
   }

   impl IngestStats {
       pub fn calculate_cost(&mut self, model: &str) {
           let cost_per_1k = match model {
               "gpt-4o-mini" => 0.00015,
               "gpt-4o" => 0.005,
               _ => 0.001,
           };
           self.estimated_cost_usd = (self.tokens_used as f64 / 1000.0) * cost_per_1k;
       }
   }
   ```

2. **Budget Limits**

   ```rust
   pub struct PipelineConfig {
       /// Maximum cost per document in USD
       pub max_cost_per_doc: Option<f64>, // Default: None
   }

   async fn process(&self, doc: &str) -> Result<Stats> {
       let mut stats = Stats::default();

       while let Some(chunk) = chunks.next() {
           stats.merge(self.process_chunk(chunk).await?);

           if let Some(max) = self.config.max_cost_per_doc {
               if stats.estimated_cost_usd > max {
                   tracing::warn!("Budget exceeded, stopping processing");
                   break;
               }
           }
       }

       Ok(stats)
   }
   ```

3. **Cost-Aware Defaults**
   - Default gleaning to 1 pass (not 2-3)
   - Only enable LLM summarization above description length threshold

---

## Monitoring & Observability

### Key Metrics to Track

1. **Ingestion Metrics**

   - `edgequake_ingest_duration_seconds`
   - `edgequake_gleaning_iterations_total`
   - `edgequake_entities_extracted_total`
   - `edgequake_llm_tokens_used_total`
   - `edgequake_llm_cost_usd_total`

2. **Query Metrics**

   - `edgequake_query_duration_seconds`
   - `edgequake_rerank_duration_seconds`
   - `edgequake_degree_ranking_duration_seconds`
   - `edgequake_context_tokens_total`

3. **Error Metrics**
   - `edgequake_llm_errors_total{type="rate_limit"}`
   - `edgequake_reranker_errors_total`
   - `edgequake_storage_errors_total`

### Alerting Rules

```yaml
# prometheus-rules.yml
groups:
  - name: edgequake
    rules:
      - alert: HighLLMErrorRate
        expr: rate(edgequake_llm_errors_total[5m]) > 0.1
        for: 5m
        annotations:
          summary: "High LLM error rate"

      - alert: SlowQueries
        expr: histogram_quantile(0.95, edgequake_query_duration_seconds) > 5
        for: 5m
        annotations:
          summary: "Query P95 latency exceeded 5s"

      - alert: CostSpike
        expr: increase(edgequake_llm_cost_usd_total[1h]) > 10
        for: 15m
        annotations:
          summary: "LLM costs spiked >$10/hour"
```

---

## Cross-References

- **Phase 1**: [01-phase-1-wire-features.md](01-phase-1-wire-features.md)
- **Phase 2**: [01-phase-2-api-integration.md](01-phase-2-api-integration.md)
- **Phase 3**: [02-phase-3-ui-integration.md](02-phase-3-ui-integration.md)
- **Testing**: [03-phase-4-testing.md](03-phase-4-testing.md)
