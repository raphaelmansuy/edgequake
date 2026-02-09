# Performance Tuning Guide

> **Optimizing EdgeQuake for Production Workloads**

This guide covers performance tuning strategies for EdgeQuake deployments.

---

## Performance Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                 PERFORMANCE BOTTLENECKS                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Request Latency Breakdown (typical hybrid query):              │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Phase          │ Time    │ Bottleneck                    │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │ Embedding      │ 50ms    │ LLM API latency               │   │
│  │ Vector Search  │ 20ms    │ pgvector index                │   │
│  │ Graph Traverse │ 30ms    │ Apache AGE queries            │   │
│  │ LLM Generation │ 2000ms  │ Token generation (dominant)   │   │
│  │ Network/Parse  │ 50ms    │ Serialization                 │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │ TOTAL          │ ~2150ms │ LLM is 93% of latency         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  Key Insight: Optimizing LLM selection has largest impact       │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Quick Wins

### 1. Choose Faster LLM Models

| Model                | Latency (TTFT) | Cost | Quality   |
| -------------------- | -------------- | ---- | --------- |
| gpt-4o               | 500ms          | $$$$ | Excellent |
| gpt-4o-mini          | 200ms          | $    | Very Good |
| gemma3:12b (Ollama)  | 100ms          | Free | Good      |
| llama3.2:3b (Ollama) | 50ms           | Free | Moderate  |

**Recommendation**: Use `gpt-4o-mini` for production (best latency/quality ratio)

```bash
export EDGEQUAKE_LLM_MODEL=gpt-4o-mini
```

### 2. Reduce Context Size

Smaller context = faster LLM processing:

```bash
# Query with fewer chunks
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "...", "max_chunks": 5, "max_entities": 5}'
```

**Default vs Optimized**:
| Setting | Default | Optimized |
|---------|---------|-----------|
| `max_chunks` | 20 | 5-10 |
| `max_entities` | 10 | 3-5 |
| `max_relationships` | 20 | 5-10 |

### 3. Use Appropriate Query Mode

| Mode     | Speed   | Use Case               |
| -------- | ------- | ---------------------- |
| `naive`  | Fastest | Simple factual queries |
| `local`  | Fast    | Entity-focused queries |
| `hybrid` | Medium  | General queries        |
| `global` | Slow    | Overview/theme queries |

```bash
# Fast mode for simple queries
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "What is X?", "mode": "naive"}'
```

---

## Document Processing Optimization

### Worker Configuration

```bash
# Default: Uses all CPU cores
# For I/O bound workloads (LLM API calls), use 2x cores
export WORKER_THREADS=8  # For 4-core machine
```

### Chunk Size Tuning

```
┌─────────────────────────────────────────────────────────────────┐
│                 CHUNK SIZE TRADEOFFS                             
├─────────────────────────────────────────────────────────────────┤
│                                                                  
│  Small chunks (256 tokens):                                      
│  ✅ More precise retrieval                                       
│  ✅ Lower token cost per extraction                              
│  ❌ More LLM calls (slower processing)                          
│  ❌ Less context per chunk                                       
│                                                                   
│  Large chunks (1024 tokens):                                     
│  ✅ Fewer LLM calls (faster processing)                         
│  ✅ Better context preservation                                  
│  ❌ Less precise retrieval                                       
│  ❌ Higher token cost per extraction                             
│                                                                   
│  Recommendation: 1200 tokens (default, balanced)                
│                                                                 
└─────────────────────────────────────────────────────────────────┘
```

### Batch Processing

For bulk uploads, process in batches:

```bash
# Upload via batch endpoint (more efficient)
curl -X POST http://localhost:8080/api/v1/documents/upload/batch \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.pdf" \
  -F "files=@doc3.pdf"
```

---

## Database Optimization

### PostgreSQL Configuration

**postgresql.conf** tuning for EdgeQuake:

```ini
# Memory (adjust for your RAM)
shared_buffers = 4GB                  # 25% of RAM
effective_cache_size = 12GB           # 75% of RAM
work_mem = 256MB                      # For complex queries
maintenance_work_mem = 1GB            # For indexing

# Connections
max_connections = 200                 # Match app pool size

# Write Ahead Log
wal_buffers = 64MB
checkpoint_completion_target = 0.9

# Query Planning
random_page_cost = 1.1                # For SSD storage
effective_io_concurrency = 200        # For SSD storage

# Parallel Query
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
```

### Connection Pooling

Use PgBouncer for high-concurrency:

```ini
# pgbouncer.ini
[databases]
edgequake = host=localhost port=5432 dbname=edgequake

[pgbouncer]
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 50
reserve_pool_size = 10
```

**Connection String**:

```bash
# Via PgBouncer (port 6432)
DATABASE_URL="postgresql://user:pass@localhost:6432/edgequake"
```

### pgvector Index Tuning

```sql
-- Check current index
\d embeddings

-- Optimal HNSW parameters for performance
CREATE INDEX CONCURRENTLY embeddings_vector_idx
ON embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- For higher recall (slower)
-- WITH (m = 32, ef_construction = 128);
```

**Search Quality vs Speed**:

| ef_search | Recall | Latency |
| --------- | ------ | ------- |
| 40        | 95%    | 10ms    |
| 100       | 98%    | 20ms    |
| 200       | 99%    | 40ms    |

```sql
-- Set search quality at runtime
SET hnsw.ef_search = 100;
```

### Apache AGE Tuning

```sql
-- Ensure graph is loaded in memory
SET search_path = ag_catalog, "$user", public;
LOAD 'age';

-- Index commonly filtered properties
SELECT create_vlabel('edgequake_graph', 'Entity');
SELECT create_elabel('edgequake_graph', 'Relationship');
```

---

## Query Optimization

### Embedding Caching

EdgeQuake caches embeddings for repeated queries:

```
┌─────────────────────────────────────────────────────────────────┐
│                 QUERY CACHING                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Query "What is X?" ──→ [Embedding Cache] ──→ Vector Search     │
│                              │                                  │
│                    Cache Hit: 0ms                               │
│                    Cache Miss: 50ms                             │
│                                                                 │
│  Cache is in-memory, cleared on restart                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Reranking Strategy

Reranking improves quality but adds latency:

```bash
# Disable reranking for faster queries
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "...", "enable_rerank": false}'

# Or use smaller rerank set
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "...", "rerank_top_k": 3}'
```

| Reranking | Latency | Quality  |
| --------- | ------- | -------- |
| Disabled  | -100ms  | Baseline |
| Top 3     | +30ms   | +5%      |
| Top 5     | +50ms   | +8%      |
| Top 10    | +100ms  | +10%     |

### Query Prefetching

For chat applications, prefetch likely follow-up queries:

```javascript
// Client-side optimization
async function queryWithPrefetch(query) {
  const response = await fetch("/api/v1/query", {
    method: "POST",
    body: JSON.stringify({ query }),
  });

  // Prefetch entity expansions in background
  const entities = extractEntities(await response.json());
  entities.slice(0, 3).forEach((entity) => {
    fetch(`/api/v1/graph/entities/${entity}/neighborhood`);
  });
}
```

---

## LLM Provider Optimization

### OpenAI Optimization

```bash
# Use streaming for faster time-to-first-token
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Accept: text/event-stream" \
  -d '{"query": "..."}'
```

### Ollama Optimization

**GPU Acceleration**:

```bash
# Ensure CUDA is available
nvidia-smi

# Set GPU layers (more = faster, more VRAM)
export OLLAMA_NUM_GPU=50
ollama serve
```

**Model Quantization**:
| Quantization | Speed | Quality | VRAM |
|--------------|-------|---------|------|
| Q4_K_M | Fastest | Good | 4GB |
| Q5_K_M | Fast | Better | 5GB |
| Q8_0 | Slow | Best | 8GB |
| FP16 | Slowest | Reference | 16GB |

```bash
# Download quantized model
ollama pull gemma3:12b-q4_K_M
```

### Local vs Cloud Latency

```
┌─────────────────────────────────────────────────────────────────┐
│                 LATENCY COMPARISON                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Local Ollama (RTX 4090):                                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Time to First Token: 50ms                                │   │
│  │ Token Generation: 100 tokens/sec                         │   │
│  │ Total (500 tokens): 5.05s                                │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  OpenAI gpt-4o-mini:                                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Time to First Token: 200ms                               │   │
│  │ Token Generation: 80 tokens/sec                          │   │
│  │ Network overhead: 50ms                                   │   │
│  │ Total (500 tokens): 6.5s                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Verdict: Local GPU is faster for inference-heavy workloads     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Scaling Strategies

### Horizontal Scaling

```
┌─────────────────────────────────────────────────────────────────┐
│                 HORIZONTAL ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                      Load Balancer                              │
│                           │                                     │
│         ┌─────────────────┼─────────────────┐                   │
│         ↓                 ↓                 ↓                   │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐            │
│  │ EdgeQuake 1 │   │ EdgeQuake 2 │   │ EdgeQuake 3 │            │
│  │  (Queries)  │   │  (Queries)  │   │ (Processing)│            │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘            │
│         │                 │                 │                   │
│         └─────────────────┼─────────────────┘                   │
│                           ↓                                     │
│                    ┌─────────────┐                              │
│                    │ PostgreSQL  │                              │
│                    │  + Replicas │                              │
│                    └─────────────┘                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Kubernetes HPA**:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: edgequake-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: edgequake
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Read Replicas

Separate read and write workloads:

```bash
# Primary for writes
DATABASE_URL="postgresql://user:pass@primary:5432/edgequake"

# Replica for reads (queries)
DATABASE_READ_URL="postgresql://user:pass@replica:5432/edgequake"
```

---

## Monitoring Performance

### Key Metrics

| Metric                | Target     | Alert        |
| --------------------- | ---------- | ------------ |
| p50 query latency     | <2s        | >5s          |
| p99 query latency     | <10s       | >30s         |
| Processing throughput | >1 doc/min | <0.5 doc/min |
| Error rate            | <1%        | >5%          |
| DB connection pool    | <80%       | >90%         |

### Prometheus Queries

```promql
# Query latency percentiles
histogram_quantile(0.99,
  rate(edgequake_query_duration_seconds_bucket[5m])
)

# Processing throughput
rate(edgequake_documents_processed_total[5m])

# Error rate
rate(edgequake_query_errors_total[5m])
  / rate(edgequake_query_total[5m])
```

### Benchmarking

```bash
# Run built-in benchmarks
cargo bench

# Results:
# vector_search          10.2 ms/iter
# graph_traverse         5.1 ms/iter
# entity_extraction     150 ms/iter (mock LLM)
```

---

## Performance Checklist

### Pre-Optimization

- [ ] Baseline metrics recorded
- [ ] Bottleneck identified (usually LLM)
- [ ] Resource monitoring in place

### Quick Wins

- [ ] Using gpt-4o-mini (or faster model)
- [ ] Context size reduced (max_chunks ≤ 10)
- [ ] Appropriate query mode selected
- [ ] Streaming enabled for chat

### Database

- [ ] PostgreSQL tuned for RAM
- [ ] pgvector HNSW index created
- [ ] Connection pooling enabled
- [ ] Read replicas for high load

### Scaling

- [ ] Horizontal scaling configured
- [ ] Auto-scaling rules defined
- [ ] Load testing completed
- [ ] Graceful degradation planned

---

## Troubleshooting Slow Queries

### Debug Query Timing

```bash
# Add timing to response
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "...", "debug": true}'
```

**Response**:

```json
{
  "answer": "...",
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 123,
    "generation_time_ms": 2890,
    "total_time_ms": 3058
  }
}
```

### Common Causes

| Symptom               | Cause           | Fix                     |
| --------------------- | --------------- | ----------------------- |
| Slow embedding        | Cold start      | Warm up with test query |
| Slow retrieval        | Missing index   | Create HNSW index       |
| Slow generation       | Large context   | Reduce max_chunks       |
| Slow generation       | Slow model      | Switch to faster model  |
| High latency variance | Connection pool | Enable PgBouncer        |

---

## See Also

- [Configuration Reference](../operations/configuration.md) - All settings
- [Deployment Guide](../operations/deployment.md) - Production setup
- [Monitoring Guide](../operations/monitoring.md) - Observability
