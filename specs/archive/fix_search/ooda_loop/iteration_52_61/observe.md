# OODA Loop 52-61: OBSERVE Phase

> **Date:** 2026-01-06  
> **Mission:** Close the gap with LightRAG - implement 10 iterations to improve hybrid/global mode quality

---

## Current State Observation

### 1. Query Mode Analysis

| Mode   | Keyword Extraction | Entity Search | Relationship Search | Current Issue         |
| ------ | ------------------ | ------------- | ------------------- | --------------------- |
| Local  | ❌ Not used        | ✅ Embedding  | ❌ None             | Works but no keywords |
| Global | ✅ High-level      | ✅ From edges | ✅ Embedding        | Keywords not helping  |
| Hybrid | ❌/✅ Mixed        | ✅ Local+Glob | ✅ From global      | Local dominates       |
| Mix    | ❌ Not used        | ✅ Local      | ❌ None             | Missing keywords      |
| Naive  | ❌ Not used        | ❌ None       | ❌ None             | Simple chunk search   |

### 2. French Query Test Results

```
Query: "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas.
        Concrètement, qu'est-ce que la plateforme STLA Medium du E-3008 m'apporte
        de plus en termes d'efficience réelle sur autoroute et de vitesse de recharge
        par rapport au chinois ?"
```

| Mode   | Answer Length | Quality    | Root Cause                         |
| ------ | ------------- | ---------- | ---------------------------------- |
| Local  | 2086 chars    | ✅ Good    | Embedding finds entities directly  |
| Hybrid | 628 chars     | ⚠️ Generic | Global keywords not helping        |
| Global | 639 chars     | ⚠️ Generic | Keywords not finding relationships |

### 3. Code Path Analysis

#### Local Mode (query.rs:225-415)

```rust
async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // ❌ NO keyword extraction

    // 1. Embed query directly
    let query_embeddings = self.embedding.embed(&[query.to_string()]).await?;

    // 2. Vector search for entities
    let entity_results = self.vector_storage.query(query_embedding, params.top_k, None).await?;

    // 3. Filter by type == "entity"
    for result in &entity_results {
        if result.metadata.get("type") != Some("entity") { continue; }
        entity_ids.push(result.id.clone());
    }

    // 4. Batch fetch nodes (good!)
    let nodes_map = self.graph_storage.get_nodes_batch(&entity_ids).await?;
}
```

#### Global Mode (query.rs:423-590)

```rust
async fn query_global(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // ✅ Keyword extraction
    let keyword_extractor = KeywordExtractor::new(Arc::clone(&self.llm));
    let keywords = keyword_extractor.extract(query).await?;

    // Use high-level keywords
    let keyword_texts: Vec<String> = keywords.high_level.clone();

    // Embed keywords (or fall back to query)
    let search_texts = if keyword_texts.is_empty() { vec![query] } else { keyword_texts };
    let keyword_embeddings = self.embedding.embed(&search_texts).await?;

    // ✅ Search relationships using keywords
    for keyword_embedding in &keyword_embeddings {
        let results = self.vector_storage.query(keyword_embedding, per_keyword_k, None).await?;
        // Filter by type == "relationship"
    }
}
```

#### Hybrid Mode (query.rs:778-870)

```rust
async fn query_hybrid(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // Run both modes
    let local_result = self.query_local(query, params).await?;
    let global_result = self.query_global(query, params).await?;

    // ⚠️ Simple concatenation - LOCAL FIRST
    let mut merged_entities = local_result.context.entities;  // Local comes first!
    let seen_entity_names: HashSet<_> = merged_entities.iter().map(|e| e.name.clone()).collect();

    // Global entities added only if not seen
    for entity in global_result.context.entities {
        if !seen_entity_names.contains(&entity.name) {
            merged_entities.push(entity);
        }
    }
}
```

### 4. Data Availability Verification

**Files in specs/fix_search/data:**

- ✅ `EF-Extract-BYD-Seal.md` (254 lines) - BYD Seal U specs
- ✅ `EF-extract-CT_3008.md` (1298 lines) - E-3008 complete specs
- ✅ `EF-extract-3008.md` (117 lines) - E-3008 summary

**Key Data Present:**
| Aspect | BYD Seal U (PHEV) | E-3008 Electric | In Files |
|--------|-------------------|-----------------|----------|
| Battery | 18.3-26.6 kWh LFP | 73-97 kWh NMC | ✅ |
| Consumption | 17.9-23.5 kWh/100km | 16.8-18.1 kWh/100km | ✅ |
| DC Charging | 18 kW, 35-55 min | 160 kW, 27-30 min | ✅ |
| Range | 70-125 km electric | 513-701 km | ✅ |

**Missing Data:**

- ❌ "STLA Medium platform" not explicitly mentioned
- ❌ Highway-specific consumption (only WLTP mixed)

### 5. Gap Analysis vs LightRAG

| Feature                  | LightRAG                | EdgeQuake             | Gap Severity |
| ------------------------ | ----------------------- | --------------------- | ------------ |
| Local mode keywords      | ✅ ll_keywords          | ❌ None               | 🔴 Critical  |
| Keyword embedding search | ✅ Multi-vector         | ⚠️ Single or fallback | 🟠 Moderate  |
| Context merging          | ✅ Round-robin          | ❌ Concatenation      | 🟠 Moderate  |
| Token truncation         | ✅ 4-stage              | ❌ None               | 🟡 Minor     |
| Reranking                | ✅ Cohere/cross-encoder | ❌ None               | 🟡 Minor     |

---

## Key Observations Summary

1. **Local mode ignores keywords** - Direct embedding is effective but misses semantic understanding
2. **Hybrid mode is local-biased** - Entities from local are prioritized, global is secondary
3. **Global mode uses keywords correctly** - But relationship search may not find entities effectively
4. **Data exists** - The French query SHOULD be answerable with available specs
5. **Round-robin missing** - LightRAG interleaves local+global, EdgeQuake concatenates

---

## Next Steps (Orient Phase)

1. Add keyword extraction to local mode
2. Implement round-robin context merging in hybrid mode
3. Add multi-vector search combining query + keywords
4. Test with French query to validate improvements
