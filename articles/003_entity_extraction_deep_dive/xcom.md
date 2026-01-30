# X.com Thread: Entity Extraction Deep Dive

## Tweet 1 (Hook)

We tried JSON for LLM entity extraction.

10-20% failure rate.

One missing bracket = zero entities extracted.

So we built something better.

Here's how EdgeQuake extracts knowledge from documents 🧵

---

## Tweet 2

The problem with JSON:

```json
{
  "entities": [
    {"name": "SARAH_CHEN", ...},
    {"name": "MIT", ...  ← Missing ]
  ]
}
```

Parse error. Total failure.

LLMs are great at understanding.
They're terrible at perfect JSON.

---

## Tweet 3

The solution: Tuple-delimited format

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Researcher
entity<|#|>MIT<|#|>ORG<|#|>Institution
relation<|#|>SARAH<|#|>MIT<|#|>works_at<|#|>...
<|COMPLETE|>
```

Line-by-line parsing.
Skip bad lines.
Keep the good ones.

99% parse success.

---

## Tweet 4

Why LLMs instead of traditional NER?

Traditional NER:
"John Smith" → [PERSON]

LLM extraction:
"John Smith" → [PERSON]

- "Lead climate researcher"
- WORKS_AT → MIT
- COLLABORATES_WITH → Sarah Chen

2-3x more entities.
Relationships included.
No training needed.

---

## Tweet 5

The extraction prompt matters.

We tell the LLM:

"You are a Knowledge Graph Specialist.
Extract entities and relationships.
Decompose N-ary to binary pairs.
Use this tuple format.
Signal completion with <|COMPLETE|>"

Role definition focuses the model.

---

## Tweet 6

But first-pass extraction isn't enough.

Complex documents have buried entities.

Solution: Gleaning

Pass 1: 8 entities
Gleaning prompt: "Find what you missed"
Pass 2: +3 more entities

Total: 11 entities
Improvement: +37%

---

## Tweet 7

The gleaning loop:

```
Input Text
    │
    ▼
First Pass → 8 entities
    │
    ▼
Threshold Check
    │
    ▼ (below threshold)
Gleaning Prompt
    │
    ▼
Second Pass → +3 entities
    │
    ▼
Total: 11 entities
```

Configurable depth vs cost trade-off.

---

## Tweet 8

Now for the dirty secret: duplicate entities.

Raw LLM output:

- "John Doe"
- "john doe"
- "JOHN DOE"

= 3 nodes in your graph.

Wrong.

Without normalization, your knowledge graph explodes.

---

## Tweet 9

EdgeQuake normalizes everything:

"John Doe" → JOHN_DOE
"the company" → COMPANY
"Dr. Sarah Chen" → DR_SARAH_CHEN

Rules:
• UPPERCASE
• Spaces → underscores
• Remove articles
• Handle possessives

Deduplication: 40-67%

---

## Tweet 10

Before/After normalization:

BEFORE:
• "John Doe" (node 1)
• "john doe" (node 2)
• "JOHN DOE" (node 3)
• "Sarah Chen" (node 4)
• "Dr. S. Chen" (node 5)

AFTER:
• JOHN_DOE (merged)
• SARAH_CHEN (merged)

5 nodes → 2 nodes. Clean.

---

## Tweet 11

The complete pipeline:

Document
↓
Chunking (600-1200 tokens)
↓
LLM Extract (tuple format)
↓
Parsing (line-by-line)
↓
Gleaning (+20-30%)
↓
Normalization (40-67% dedup)
↓
Knowledge Graph

---

## Tweet 12

Real production numbers:

• Entities per 10k doc: 15-25
• Relationships: 10-20
• Extraction time: 2-10s
• Parse success: 99%
• Gleaning boost: +20-37%
• Deduplication: 40-67%

Compare to NER: 2-3x more entities with relationships.

---

## Tweet 13

The best part?

Domain-agnostic.

Works on:
• Legal contracts
• Medical records
• Technical papers
• Financial reports

No training. No fine-tuning.

Just prompt engineering and tuple parsing.

---

## Tweet 14

EdgeQuake is open source.

The full entity extraction pipeline:
github.com/raphaelmansuy/edgequake

Tuple parsing. Gleaning. Normalization.

Production-ready in Rust.

⭐ if you're building knowledge graphs

---

## Tweet 15 (Repost Hook)

JSON parsing with LLMs has a 10-20% failure rate.

We solved it with tuples.

Read the thread above for the full extraction pipeline.

🔄 Repost to help others avoid the JSON trap
