# Iteration 08: Decide - Query Engine Article Plan

## Article Title Options

1. "5 Query Modes for 5 Question Types: The EdgeQuake Approach"
2. "Beyond Vector Search: How Graph-Enhanced Retrieval Transforms RAG"
3. "The Query Mode Decision Tree: Matching Strategy to Question"

**Selected**: "Beyond Vector Search: How Graph-Enhanced Retrieval Transforms RAG"

## Content Strategy

### WHY (Simon Sinek)

Vector similarity search alone misses the relationships between concepts. When you ask "How does Alice collaborate with Bob?", finding chunks mentioning each person separately doesn't answer the question. You need the graph—the connections, the relationships, the context that spans documents.

### HOW (Methodology)

Multi-mode query engine with LightRAG-inspired architecture:

- Keyword extraction determines query intent
- Mode selection matches question type
- Retrieval combines vectors + graph
- Token budgeting ensures LLM success

### WHAT (Implementation)

EdgeQuake's 5 query modes:

1. Naive: Fast vector similarity
2. Local: Entity-centric graph traversal
3. Global: Community/theme-based retrieval
4. Hybrid: Best of both (default)
5. Mix: Weighted combination

## Article Structure

### Medium (2000+ words)

1. **The Vector Limitation** (400 words)
   - Why similarity isn't enough
   - Relationship questions fail
   - Context fragmentation

2. **The 5 Query Modes** (600 words)
   - Mode comparison table
   - Question type → mode mapping
   - Real examples per mode

3. **The LightRAG Algorithm** (400 words)
   - Keyword extraction (high/low level)
   - Multi-level retrieval
   - Context prioritization

4. **Token Budgeting** (300 words)
   - Graph context priority
   - Smart truncation
   - Never overflow

5. **Production Patterns** (300 words)
   - Adaptive mode selection
   - Keyword caching
   - Reranking option

### LinkedIn (<3000 chars)

Hook → 5 modes → Default choice → CTA

### X.com (12-15 tweets)

1-3. The problem with vector-only
4-8. Each mode explained
9-12. When to use each
13-15. Benchmarks & CTA

### HackerNews

Algorithm discussion, LightRAG paper, implementation tradeoffs

### Reddit

r/MachineLearning: Retrieval strategy debate
r/rust: Implementation patterns

### Substack

Story: "The query that broke our RAG system"

## Research Paper Citation

- LightRAG paper (arXiv:2410.05779): Core algorithm
- Multi-level keyword extraction approach
- Graph-enhanced retrieval strategy

## Deliverables

- [ ] medium.md
- [ ] linkedin.md
- [ ] xcom.md
- [ ] hackernews.md
- [ ] reddit.md
- [ ] substack.md
