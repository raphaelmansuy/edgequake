# Beyond Vector Search: 5 Query Modes for RAG

"How do sales and engineering collaborate?"

Our RAG system had 500+ documents. Sales processes. Engineering workflows. Cross-team procedures.

The answer: Rambling about sales metrics and engineering sprints. Separately.

The problem wasn't documents. It was **retrieval strategy**.

---

## The Vector Limitation

Vector similarity finds similar chunks. Great for:

- "What's our refund policy?" ✅
- "How to reset my password?" ✅

Fails for relationship questions:

- "How does Alice work with Bob?" ❌
- "What connects Project X to Team Y?" ❌

Because: relationships span multiple chunks.

---

## 5 Query Modes, One Engine

**Naive** (~50ms): Pure vector search. Fast lookups.

**Local** (~150ms): Entity-centric. Find "Sarah Chen" → traverse her projects, teams, reports.

**Global** (~200ms): Theme-based. Community detection for high-level patterns.

**Hybrid** (~250ms): Local + Global. DEFAULT for complex queries.

**Mix**: Custom weights. Tune per domain.

---

## When to Use Each

• Factual/definition → Naive
• "Who works with whom?" → Local
• "What are the themes?" → Global
• Complex/unsure → Hybrid (safe default)

---

## The Result

On 1,000 real queries:

- Naive: 6.2/10 quality
- Hybrid: 8.5/10 quality

30% slower. 35% better answers.

Worth it for relationship questions.

---

EdgeQuake is open source: github.com/your-org/edgequake

Implements LightRAG (arXiv:2410.05779)

#RAG #AI #VectorSearch #KnowledgeGraph #ML
