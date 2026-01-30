The biggest mistake in entity extraction?

Using JSON.

Let me explain 👇

We built EdgeQuake, a Graph-RAG framework.

Entity extraction is core.

We tried JSON output from LLMs.

10-20% failure rate.

One missing bracket = zero entities.

So we switched to tuples:

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Researcher
entity<|#|>MIT<|#|>ORG<|#|>Institution
relation<|#|>SARAH<|#|>MIT<|#|>works_at<|#|>...
<|COMPLETE|>
```

Result: 99% parse success.

Why?

→ Line-by-line parsing (streaming)
→ Skip bad lines (partial recovery)
→ No escaping nightmares

But extraction is just step 1.

Step 2: Gleaning

First pass might miss entities.

So we re-extract:
"Find entities you missed in this text..."

Result: +20-30% more knowledge.

Step 3: Normalization

Raw output:

- "John Doe"
- "john doe"
- "JOHN DOE"

= 3 nodes. Wrong.

After normalization:

- "JOHN_DOE"

= 1 node. Right.

Deduplication rate: 40-67%

The full pipeline:

```
Document
   │
   ▼
Chunking (600-1200 tokens)
   │
   ▼
LLM Extract (tuple format)
   │
   ▼
Gleaning (+20-30% coverage)
   │
   ▼
Normalization (40-67% dedup)
   │
   ▼
Knowledge Graph
```

Why LLMs over NER?

Traditional NER: "John Smith" (PERSON)

LLM extraction:

- "John Smith" (PERSON)
- "Lead climate researcher"
- WORKS_AT → MIT
- COLLABORATES_WITH → Sarah Chen

2-3x more knowledge. Relationships included.

No training required.

Works on legal, medical, technical docs.

EdgeQuake is open source.

Full technical deep-dive linked in comments 👇

What's your entity extraction approach?

#NLP #LLM #KnowledgeGraphs #GraphRAG #MachineLearning
