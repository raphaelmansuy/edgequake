# OODA Loop 5: Observe

## Re-read Mission (Every 5 Loops)
✅ Mission re-read at Loop 5 per spec requirement.

Key reminders:
- Non-regression is North Star
- Test both PostgreSQL and in-memory backends
- 30 OODA loops required
- Document thoroughly with links to real code

## Current BM25 Parameters

**Location**: `reranker.rs` BM25Reranker struct

Current defaults:
- `k1 = 1.5` - Term frequency saturation
- `b = 0.75` - Length normalization
- `delta = 0.0` - BM25+ extension (disabled by default)

## Observations

### 1. Single Preset Only
No domain-specific presets for different use cases:
- Short documents (tweets, titles)
- Long documents (articles, papers)
- Technical content (code, APIs)
- Natural language (conversational queries)

### 2. No Runtime Configuration
API layer creates reranker at startup, no per-request parameter tuning.

### 3. Research-Backed Parameter Ranges
Literature suggests:
- **k1**: [1.2, 2.0] typical, [0.5, 3.0] extreme
- **b**: [0.5, 0.9] typical, lower for short docs
- **delta**: [0.5, 1.0] for BM25+ (long doc handling)

## BM25F Assessment
BM25F (field-aware) would require:
1. Structured field input (title, body, metadata)
2. Per-field weights
3. Significant API changes

**Decision**: Defer BM25F to future iteration; focus on parameter presets.
