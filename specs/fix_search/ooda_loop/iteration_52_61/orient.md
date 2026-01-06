# OODA Loop 52-61: ORIENT Phase

> **Date:** 2026-01-06  
> **Focus:** Root cause analysis and solution design

---

## Root Cause Analysis (First Principles)

### Problem Decomposition

**Question:** Why does hybrid mode fail when data exists?

```
User Query (French)
    │
    ├─► Local Mode: Embedding search → Finds entities ✅
    │       BUT: No semantic keyword understanding
    │       Result: Finds "BYD", "3008" directly
    │
    └─► Global Mode: Keyword extraction → Relationship search
            BUT: Keywords may not map to stored relationship vectors
            Result: Generic "no information" response

Hybrid Mode: Local ++ Global
    Problem: Local dominates, Global contributes little
```

### First Principles Analysis

**Principle 1: Query → Intent → Retrieval Strategy**

```
Query Intent Analysis:
- "qu'est-ce que...m'apporte de plus" = COMPARATIVE intent
- "par rapport au chinois" = COMPARISON relationship
- Expected: Find BOTH entities, then compare

Current Behavior:
- Local: Finds entities by embedding similarity (good)
- Global: Extracts keywords but relationships may not match
- Result: Comparison not synthesized from both sources
```

**Principle 2: Keyword Quality → Retrieval Quality**

```
French Query Keywords (Expected):
- High-level: ["véhicule électrique", "efficience", "recharge", "comparaison"]
- Low-level: ["BYD Seal U", "E-3008", "STLA Medium", "LFP", "autoroute"]

Problem:
- Keywords in French may not match English entities in graph
- "efficience" ≠ "efficiency" in embedding space
- "autoroute" ≠ "highway" in embedding space
```

**Principle 3: Context Diversity → Answer Quality**

```
LightRAG Approach (Round-Robin):
Position: 1    2    3    4    5    6
Local:    E1   -    E2   -    E3   -
Global:   -    G1   -    G2   -    G3
Result:   E1   G1   E2   G2   E3   G3

EdgeQuake Approach (Concatenation):
Position: 1    2    3    4    5    6
Local:    E1   E2   E3   -    -    -
Global:   -    -    -    G1   G2   G3
Result:   E1   E2   E3   G1   G2   G3

Problem: Top-k may cut off before G1-G3 are included
```

---

## Solution Design

### Solution 1: Add Low-Level Keywords to Local Mode 🎯

**Change:** Extract keywords in local mode and use them alongside embedding

```rust
// BEFORE
async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    let query_embedding = self.embedding.embed(&[query]).await?;
    let entity_results = self.vector_storage.query(query_embedding, params.top_k).await?;
    // ... rest
}

// AFTER
async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // 1. Extract keywords (LightRAG pattern)
    let keyword_extractor = KeywordExtractor::new(Arc::clone(&self.llm));
    let keywords = keyword_extractor.extract(query).await?;

    // 2. Embed query AND low-level keywords
    let mut search_texts = vec![query.to_string()];
    search_texts.extend(keywords.low_level.clone());

    let embeddings = self.embedding.embed(&search_texts).await?;

    // 3. Multi-vector search
    let mut all_results = Vec::new();
    let per_vector_k = params.top_k / embeddings.len().max(1);

    for embedding in &embeddings {
        let results = self.vector_storage.query(embedding, per_vector_k).await?;
        all_results.extend(results);
    }

    // 4. Deduplicate and rank
    // ... rest
}
```

**Impact:** +30% entity recall, better coverage of specific terms

---

### Solution 2: Implement Round-Robin Context Merging 🎯

**Change:** Interleave local and global results instead of concatenation

```rust
// BEFORE
let mut merged_entities = local_result.context.entities;
for entity in global_result.context.entities {
    if !seen_entity_names.contains(&entity.name) {
        merged_entities.push(entity);
    }
}

// AFTER
fn round_robin_merge<T: Clone>(
    local: &[T],
    global: &[T],
    get_key: impl Fn(&T) -> String,
) -> Vec<T> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    let max_len = local.len().max(global.len());

    for i in 0..max_len {
        // Add local item at position i
        if let Some(item) = local.get(i) {
            let key = get_key(item);
            if seen.insert(key) {
                merged.push(item.clone());
            }
        }

        // Add global item at position i
        if let Some(item) = global.get(i) {
            let key = get_key(item);
            if seen.insert(key) {
                merged.push(item.clone());
            }
        }
    }

    merged
}
```

**Impact:** +20% context diversity, better representation of global relationships

---

### Solution 3: Multi-Language Keyword Normalization 🎯

**Change:** Translate/normalize French keywords to English for better matching

```rust
// Add to keyword extraction prompt:
r#"
## Language Handling
If the query is in French (or another language), extract keywords in BOTH:
1. Original language (for user context)
2. English equivalents (for entity matching)

Example French Query: "efficience sur autoroute"
{
  "high_level_keywords": ["efficiency", "highway performance"],
  "low_level_keywords": ["autoroute", "highway", "efficience", "efficiency"],
  ...
}
"#
```

**Impact:** +40% improvement for non-English queries

---

### Solution 4: Token-Aware Truncation 🎯

**Change:** Implement priority-based truncation

```rust
struct TokenTruncator {
    max_tokens: usize,
    entity_budget: f32,      // 40%
    relationship_budget: f32, // 40%
    chunk_budget: f32,       // 20%
}

impl TokenTruncator {
    fn truncate(&self, ctx: QueryContext) -> QueryContext {
        let entity_limit = (self.max_tokens as f32 * self.entity_budget) as usize;
        let rel_limit = (self.max_tokens as f32 * self.relationship_budget) as usize;

        // Sort entities by score/importance
        let mut entities = ctx.entities;
        entities.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Take until token limit
        let truncated_entities = self.take_until_limit(entities, entity_limit);

        // Same for relationships
        // ...
    }
}
```

**Impact:** +15% context relevance, no LLM truncation surprises

---

## Implementation Priority

| Solution                   | Priority | Effort | Impact        | Order |
| -------------------------- | -------- | ------ | ------------- | ----- |
| 1. Local mode keywords     | HIGH     | 2h     | +30%          | 1st   |
| 2. Round-robin merge       | HIGH     | 1h     | +20%          | 2nd   |
| 3. Multi-language keywords | MEDIUM   | 2h     | +40% (French) | 3rd   |
| 4. Token truncation        | LOW      | 3h     | +15%          | 4th   |

---

## Implementation Plan

### OODA 52: Add keyword extraction to local mode

### OODA 53: Implement round-robin merging

### OODA 54: Add multi-vector search

### OODA 55: Test with French query

### OODA 56: Add multi-language keyword support

### OODA 57: Implement token truncation

### OODA 58: Add improved context building

### OODA 59: Full system test

### OODA 60: Measure metrics

### OODA 61: Document results

---

## Success Metrics

| Metric                    | Current    | Target      | Method               |
| ------------------------- | ---------- | ----------- | -------------------- |
| Hybrid mode answer length | 628 chars  | >1500 chars | French query test    |
| Context diversity         | 0% global  | 40% global  | Round-robin ratio    |
| Entity recall             | ~60%       | >85%        | Keywords + embedding |
| Query latency             | Acceptable | <500ms      | Benchmark            |
