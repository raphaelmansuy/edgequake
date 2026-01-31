# Iteration 09: Decide - Entity Deduplication Article Plan

## Article Title Options

1. "Entity Deduplication: Building Consistent Knowledge Graphs from Messy LLM Outputs"
2. "Why Your Knowledge Graph Has 4x Too Many Nodes"
3. "The Art of Merging: How EdgeQuake Builds Unified Entity Profiles"

**Selected**: "Entity Deduplication: Building Consistent Knowledge Graphs from Messy LLM Outputs"

## Content Strategy

### WHY (Simon Sinek)

LLMs don't output consistent entity names. The same person might be "John Doe", "john doe", "John", or "Mr. Doe" depending on the context. Without normalization, your knowledge graph fragments into disconnected nodes, relationships get lost, and queries fail to find what's right in front of them.

### HOW (Methodology)

Deterministic normalization + intelligent merging:

- Normalize names before storage (UPPERCASE_UNDERSCORE)
- Merge descriptions (don't replace)
- Accumulate source references
- Optionally use LLM for smart summarization

### WHAT (Implementation)

EdgeQuake's deduplication pipeline:

1. Normalize entity name
2. Check if entity exists
3. Merge descriptions if exists
4. Accumulate source_ids
5. Update or insert node

## Article Structure

### Medium (2000+ words)

1. **The Fragmentation Problem** (400 words)
   - Same entity, 4 different names
   - Lost relationships
   - Failed queries

2. **The Normalization Algorithm** (500 words)
   - Whitespace handling
   - Prefix removal (The, A, An)
   - Possessive handling
   - UPPERCASE_UNDERSCORE format

3. **Description Merging** (400 words)
   - LLM summarization option
   - Sentence-level deduplication
   - Max length enforcement

4. **Source Lineage** (300 words)
   - Append-only source_ids
   - Full provenance tracking
   - Cascade delete support

5. **Production Results** (300 words)
   - 40% deduplication rate
   - Storage savings
   - Query accuracy improvement

### LinkedIn (<3000 chars)

Hook → Problem stats → Solution → Metrics → CTA

### X.com (12-15 tweets)

1-3. The fragmentation problem
4-7. Normalization rules
8-10. Merge strategy
11-13. Results & metrics
14-15. CTA

### HackerNews

Implementation details, edge cases, tradeoffs

### Reddit

Entity resolution techniques discussion

### Substack

"The 4 copies of John Doe ruining your RAG"

## Research Paper Citation

- LightRAG paper (arXiv:2410.05779): Entity normalization concept

## Deliverables

- [ ] medium.md
- [ ] linkedin.md
- [ ] xcom.md
- [ ] hackernews.md
- [ ] reddit.md
- [ ] substack.md
