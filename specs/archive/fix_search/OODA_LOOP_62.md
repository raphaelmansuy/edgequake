# OODA Loop 62: First Principles Fix - Keyword Validation Against Knowledge Graph

## Observe

### Problem Statement

The French challenge query "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. Concrètement, qu'est-ce que ça donne face au Peugeot E-3008 sur la plateforme STLA Medium, notamment sur autoroute ?" was returning poor results in HYBRID mode:

- HYBRID mode: 431-639 chars, "ne contient pas d'informations spécifiques"
- GLOBAL mode: 579 chars, "no information"
- LOCAL mode: 1138-1314 chars, better but still incomplete

### Initial (Wrong) Approach

First attempted prompt modification - adding "provide partial information" instructions to the LLM. This was correctly identified as **"cheating"** - a heuristic hack rather than first principles thinking.

### First Principles Root Cause Analysis

1. Query contains "STLA Medium" - a platform name that does NOT exist in the knowledge graph
2. Keyword extraction produces: `["BYD Seal U", "batterie LFP", "plateforme STLA Medium", "E-3008", "autoroute"]`
3. Embedding computation creates a combined vector: `"BYD Seal U, batterie LFP, plateforme STLA Medium, E-3008, autoroute"`
4. Non-existent term "STLA Medium" DILUTES the embedding quality
5. Vector search retrieves less relevant entities due to diluted embedding
6. LLM receives poor context and correctly reports "no information"

**The bug is NOT in the LLM prompt. The bug is in the data pipeline BEFORE embedding computation.**

## Orient

### The Real Problem

```
Query → Keywords → [PROBLEM HERE] → Embeddings → Vector Search → Context → LLM
                  ↑
                  Non-existent keywords dilute embeddings
```

### Solution Design

Add a validation step between keyword extraction and embedding computation:

1. For each extracted keyword, check if it matches any entities in the knowledge graph
2. Use `graph_storage.search_labels(keyword, limit=1)` for fuzzy matching
3. Drop keywords with zero matches
4. Only embed validated keywords
5. Fall back to original if ALL keywords dropped (edge case protection)

### Why This Works

- Entity embeddings in the vector DB were created from ACTUAL knowledge base content
- Query embeddings should only include terms that CAN match entities
- Removing non-existent terms focuses the embedding on valid search space

## Decide

### Implementation Plan

1. Add `validate_keywords()` method to `SOTAQueryEngine`
2. Call it in 3 locations: `query()`, `query_stream()`, `get_context()`
3. Use `search_labels()` from GraphStorage trait (already implemented for Postgres/Memory)
4. Add logging to track dropped vs kept keywords
5. Test with challenge queries

## Act

### Code Changes

Added to `sota_engine.rs`:

```rust
/// Validate keywords against the knowledge graph.
///
/// WHY: When a query contains terms that don't exist in the knowledge base
/// (e.g., "STLA Medium"), including them in the embedding computation dilutes
/// the semantic search and reduces retrieval quality for terms that DO exist.
async fn validate_keywords(&self, keywords: &ExtractedKeywords) -> ExtractedKeywords {
    if keywords.low_level.is_empty() {
        return keywords.clone();
    }

    let mut validated_low_level = Vec::new();
    let mut dropped_keywords = Vec::new();

    for keyword in &keywords.low_level {
        let matches = self.graph_storage.search_labels(keyword, 1).await;
        match matches {
            Ok(labels) if !labels.is_empty() => validated_low_level.push(keyword.clone()),
            _ => dropped_keywords.push(keyword.clone()),
        }
    }

    if !dropped_keywords.is_empty() {
        tracing::info!(
            dropped = ?dropped_keywords,
            kept = ?validated_low_level,
            "Dropped keywords with no graph matches"
        );
    }

    // Edge case: if ALL dropped, fall back to original
    if validated_low_level.is_empty() {
        tracing::warn!("All keywords dropped - falling back to original");
        return keywords.clone();
    }

    ExtractedKeywords::new(
        keywords.high_level.clone(),
        validated_low_level,
        keywords.query_intent,
    )
}
```

### Observed Logs After Fix

```
Dropped keywords with no graph matches dropped=["STLA Medium"] kept=["E-3008", "Peugeot", "battery"]
Dropped keywords with no graph matches dropped=["plateforme STLA Medium", "autoroute"] kept=["BYD Seal U", "batterie LFP", "E-3008"]
```

## Results

### Before Fix

| Query              | Mode   | Response Length | Quality               |
| ------------------ | ------ | --------------- | --------------------- |
| French Challenge   | HYBRID | 431-639 chars   | ❌ "no information"   |
| French Challenge   | GLOBAL | 579 chars       | ❌ "no information"   |
| French Challenge   | LOCAL  | 1138-1314 chars | ⚠️ partial            |
| STLA Medium E-3008 | HYBRID | 298 chars       | ❌ "does not contain" |

### After Fix

| Query              | Mode   | Response Length | Quality           |
| ------------------ | ------ | --------------- | ----------------- |
| French Challenge   | HYBRID | 2226 chars      | ✅ detailed specs |
| French Challenge   | GLOBAL | 1464 chars      | ✅ detailed specs |
| French Challenge   | LOCAL  | 2102 chars      | ✅ detailed specs |
| STLA Medium E-3008 | HYBRID | 1279 chars      | ✅ battery specs  |

### Improvement

- **HYBRID mode**: 639 → 2226 chars = **3.5x improvement**
- **GLOBAL mode**: 579 → 1464 chars = **2.5x improvement**
- Quality changed from "no information" to detailed technical specs

## Key Insight

**The LLM was correct** - it honestly reported "no information" when the context was poor. The fix was NOT to make the LLM lie better, but to give it better context by:

1. Validating keywords exist in knowledge graph
2. Dropping non-existent terms before embedding
3. Focusing the semantic search on valid entity space

This is **first principles thinking**: fix the data flow, not the output formatting.

## Files Modified

- `edgequake/crates/edgequake-query/src/sota_engine.rs` (+60 lines)
  - Added `validate_keywords()` method
  - Called in `query()`, `query_stream()`, `get_context()`

## Lessons Learned

1. Prompt engineering is a band-aid, not a fix
2. The real bug is often upstream in the data pipeline
3. Check if query terms actually exist in the knowledge base
4. Validate before compute, not after retrieve
