# Entity Deduplication: The Hidden Cost of LLM Outputs

We ingested 1,000 documents. Extracted 12,450 entities.

Then I searched for "John Doe" and found... fragments.

Same person. 4 different nodes:

- "John Doe"
- "john doe"
- "JOHN DOE"
- "The John Doe"

**40% of our entities were duplicates.**

---

## The Cascade of Problems

• Lost relationships (edges don't connect)
• Failed queries (exact match misses variants)
• Inflated storage (40% waste)
• Slower traversal (more nodes, same info)

---

## The Solution: Normalize Before Storage

EdgeQuake transforms every entity name to a canonical form:

```
"John Doe"      → JOHN_DOE
"john doe"      → JOHN_DOE
"The John Doe"  → JOHN_DOE
"  Sarah  Chen  " → SARAH_CHEN
```

All variants → same node → relationships connected.

---

## Merge, Don't Replace

When same entity found in multiple docs:

Doc 1: "Chen is an engineer"
Doc 2: "Chen leads the ML team"

Result: "Chen is an engineer and leads the ML team"

Information accumulates. Nothing lost.

---

## Results

Before: 12,450 nodes, 2.1 edges/node
After: 7,470 nodes, 3.5 edges/node

40% deduplication.
67% more edges per node.
Query accuracy: 5.8/10 → 8.2/10

---

EdgeQuake handles this automatically: github.com/your-org/edgequake

#RAG #KnowledgeGraph #AI #DataQuality #ML
