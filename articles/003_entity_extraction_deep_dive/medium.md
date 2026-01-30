# How EdgeQuake Extracts Knowledge from Documents

_LLMs as librarians: The entity extraction deep-dive_

---

## The Knowledge Extraction Challenge

You've got documents. Hundreds of them. Maybe thousands.

Inside those documents is knowledge: people, organizations, concepts, and the relationships between them. But that knowledge is locked up in paragraphs, buried in PDFs, scattered across formats.

Traditional approaches to extraction have two paths:

1. **Named Entity Recognition (NER)**: Fast, but shallow. Finds "John Smith" but not what makes John important.
2. **Manual extraction**: Accurate, but doesn't scale. Good luck with 10,000 documents.

EdgeQuake takes a third path: **LLMs as extraction engines**.

---

## The LLM Advantage

Here's the insight: Large Language Models are incredible at understanding context. They don't just recognize "John Smith" as a person—they understand that John is a _lead researcher_ who _collaborated_ with Sarah on the _climate paper_.

```
┌─────────────────────────────────────────────────────────────────┐
│                 EXTRACTION APPROACHES COMPARED                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Traditional NER:                                                │
│  "John Smith works at MIT on climate research with Sarah Chen"  │
│       │                │                           │             │
│       ▼                ▼                           ▼             │
│  [PERSON: John]   [ORG: MIT]               [PERSON: Sarah]      │
│                                                                   │
│  ❌ Lost: "works at", "research", "with" relationships          │
│                                                                   │
│  ────────────────────────────────────────────────────────────── │
│                                                                   │
│  LLM-Based Extraction:                                           │
│  "John Smith works at MIT on climate research with Sarah Chen"  │
│       │                │                           │             │
│       ▼                ▼                           ▼             │
│  [JOHN_SMITH]     [MIT]                      [SARAH_CHEN]       │
│   (PERSON)        (ORG)                       (PERSON)          │
│   "Lead climate   "Research                  "Climate           │
│    researcher"    institution"               collaborator"      │
│       │                                           │              │
│       └────── WORKS_AT ────────────────┬──────────┘              │
│       └────── COLLABORATES_WITH ───────┘                         │
│       └────── RESEARCHES → CLIMATE_SCIENCE ─────────             │
│                                                                   │
│  ✅ Entities with descriptions + relationships extracted        │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

The LLM gives us:

- **Rich descriptions** (not just labels)
- **Explicit relationships** (not just co-occurrence)
- **Semantic understanding** (context-aware extraction)

And the best part? **No training required**. It works on legal documents, medical records, technical papers—any domain.

---

## The Tuple Format: Why We Ditched JSON

Here's something we learned the hard way: **JSON parsing with LLMs is fragile**.

Ask an LLM to output JSON, and 10-20% of the time you'll get:

- Missing closing brackets
- Unescaped quotes in descriptions
- Trailing commas
- Truncated output

One malformed character and your entire extraction fails.

EdgeQuake uses a **tuple-delimited format** instead:

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Lead researcher at Quantum Lab
entity<|#|>MIT<|#|>ORGANIZATION<|#|>Research institution in Cambridge
entity<|#|>NEURAL_NETWORK<|#|>CONCEPT<|#|>Machine learning architecture
relation<|#|>SARAH_CHEN<|#|>MIT<|#|>works_at<|#|>Sarah works at MIT
relation<|#|>SARAH_CHEN<|#|>NEURAL_NETWORK<|#|>researches<|#|>Sarah researches neural networks
<|COMPLETE|>
```

### Why Tuples Win

```
┌─────────────────────────────────────────────────────────────────┐
│                    JSON vs TUPLE COMPARISON                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  JSON Format:                                                    │
│  {                                                               │
│    "entities": [                                                 │
│      {"name": "SARAH_CHEN", "type": "PERSON", ...},             │
│      {"name": "MIT", "type": "ORGANIZATION", ...}  ← Missing ]  │
│    ]                                                             │
│  }                                                               │
│                                                                   │
│  ❌ Result: Parse error. ZERO entities extracted.               │
│                                                                   │
│  ────────────────────────────────────────────────────────────── │
│                                                                   │
│  Tuple Format:                                                   │
│  entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>...                       │
│  entity<|#|>MIT<|#|>ORGANIZATION<|#|>...                        │
│  entity<|#|>GARBLED_OUTPUT ← Bad line                           │
│  entity<|#|>NEURAL_NETWORK<|#|>CONCEPT<|#|>...                  │
│                                                                   │
│  ✅ Result: 3 valid entities. Bad line skipped.                 │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

| Aspect           | JSON                       | Tuples                |
| ---------------- | -------------------------- | --------------------- |
| Streaming        | ❌ Need complete structure | ✅ Line-by-line       |
| Partial recovery | ❌ All or nothing          | ✅ Skip bad lines     |
| Escaping         | ❌ Quote/backslash issues  | ✅ No special chars   |
| LLM reliability  | ❌ 10-20% failure rate     | ✅ ~99% parse success |

This isn't theoretical—it's battle-tested from the LightRAG research and thousands of production extractions.

---

## The Extraction Prompt

The quality of extraction depends on the prompt. Here's what EdgeQuake sends to the LLM:

```
┌─────────────────────────────────────────────────────────────────┐
│                    EXTRACTION PROMPT STRUCTURE                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  SYSTEM PROMPT:                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Role: Knowledge Graph Specialist                            ││
│  │                                                              ││
│  │ Entity Instructions:                                         ││
│  │ - Identify meaningful entities                               ││
│  │ - Use types: PERSON, ORG, LOCATION, CONCEPT...              ││
│  │ - Provide concise descriptions                               ││
│  │                                                              ││
│  │ Relationship Instructions:                                   ││
│  │ - Identify direct relationships                              ││
│  │ - Decompose N-ary to binary pairs                           ││
│  │ - Use keywords to summarize                                  ││
│  │                                                              ││
│  │ Output Format:                                               ││
│  │ - entity<|#|>name<|#|>type<|#|>description                  ││
│  │ - relation<|#|>source<|#|>target<|#|>keywords<|#|>desc      ││
│  │ - <|COMPLETE|> when done                                     ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                   │
│  USER PROMPT:                                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Entity types: [PERSON, ORG, LOCATION, CONCEPT, ...]         ││
│  │                                                              ││
│  │ Input text:                                                  ││
│  │ "Sarah Chen works at Quantum Dynamics Lab. She authored..."  ││
│  │                                                              ││
│  │ Extract entities and relationships.                          ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

Key prompt engineering choices:

1. **Role definition**: "Knowledge Graph Specialist" focuses the LLM
2. **N-ary decomposition**: "Alice, Bob, and Carol collaborated" becomes 3 binary relationships
3. **Consistent naming**: Prevents duplicates like "Sarah" vs "Dr. Chen"
4. **Completion signal**: `<|COMPLETE|>` tells us when extraction is done

---

## Gleaning: Finding What Was Missed

First-pass extraction is good. But it's not perfect.

Complex documents have entities buried in context. The LLM might miss them on the first pass—especially with dense technical content.

EdgeQuake implements **gleaning**: iterative re-extraction for completeness.

```
┌─────────────────────────────────────────────────────────────────┐
│                    GLEANING LOOP                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   Input Text                                                     │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────────┐                                                │
│  │ First Pass   │ → 8 entities, 5 relationships                 │
│  │ Extraction   │                                                │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │ Threshold    │ "Did we find enough?"                         │
│  │ Check        │ (configurable: min entities, min relations)   │
│  └──────┬───────┘                                                │
│         │                                                        │
│         │ NO: Below threshold                                    │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │ Gleaning     │ "Find missed entities in this text..."        │
│  │ Prompt       │                                                │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │ Second Pass  │ → 3 more entities, 2 more relationships       │
│  │ Extraction   │                                                │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │ Merge        │ → 11 entities, 7 relationships (total)        │
│  └──────────────┘                                                │
│                                                                   │
│  Repeat up to N times (configurable)                            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

**Results from production**:

- First pass: 8 entities
- After gleaning: 11 entities
- **37% more knowledge captured**

The trade-off is LLM cost, but for high-stakes domains (legal, medical, financial), the extra coverage is worth it.

---

## Normalization: Taming the Entity Explosion

Raw LLM output is messy. The same entity might appear as:

- "John Doe"
- "john doe"
- "JOHN DOE"
- "Mr. John Doe"
- "J. Doe"

Without normalization, each becomes a separate node in your graph. Your knowledge graph explodes with duplicates.

EdgeQuake normalizes entity names to a canonical format:

```rust
normalize_entity_name("John Doe")      → "JOHN_DOE"
normalize_entity_name("the company")   → "COMPANY"
normalize_entity_name("John's team")   → "JOHN_TEAM"
normalize_entity_name("Dr. Sarah Chen") → "DR_SARAH_CHEN"
```

The rules:

1. Convert to UPPERCASE
2. Replace spaces with underscores
3. Remove articles ("the", "a", "an")
4. Handle possessives and special characters
5. Trim and clean whitespace

**Before/After Comparison**:

```
┌─────────────────────────────────────────────────────────────────┐
│                 NORMALIZATION IMPACT                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  BEFORE (Raw Extraction):                                        │
│                                                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ "John Doe"  │  │ "john doe"  │  │ "JOHN DOE"  │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│  ┌─────────────┐  ┌─────────────┐                                │
│  │ "Sarah Chen"│  │"Dr. S. Chen"│                                │
│  └─────────────┘  └─────────────┘                                │
│                                                                   │
│  = 5 nodes (duplicates!)                                        │
│                                                                   │
│  ────────────────────────────────────────────────────────────── │
│                                                                   │
│  AFTER (Normalized):                                             │
│                                                                   │
│  ┌─────────────┐         ┌─────────────┐                         │
│  │  JOHN_DOE   │         │ SARAH_CHEN  │                         │
│  │  (merged)   │         │  (merged)   │                         │
│  └─────────────┘         └─────────────┘                         │
│                                                                   │
│  = 2 nodes (clean!)                                             │
│                                                                   │
│  Deduplication: 60% reduction                                   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

In production, we see **40-67% deduplication rates**. That's the difference between a usable knowledge graph and a tangled mess.

---

## The Complete Extraction Pipeline

Putting it all together:

```
┌─────────────────────────────────────────────────────────────────┐
│              EDGEQUAKE ENTITY EXTRACTION PIPELINE                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   Document                                                       │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────────┐                                                │
│  │   Chunking   │  Split into 600-1200 token chunks             │
│  │              │  with 100-token overlap                        │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼  (parallel processing)                                 │
│  ┌──────────────┐                                                │
│  │ LLM Extract  │  Send to LLM with extraction prompt           │
│  │              │  → Tuple-delimited output                      │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │   Parsing    │  Line-by-line tuple parsing                   │
│  │              │  → Skip malformed lines                        │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │  Gleaning    │  Re-extract missed entities (optional)        │
│  │              │  → +20-30% more coverage                       │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │Normalization │  UPPERCASE_UNDERSCORE format                  │
│  │              │  → 40-67% deduplication                        │
│  └──────┬───────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │   Merging    │  Combine across chunks                        │
│  │              │  → Unified knowledge graph                     │
│  └──────────────┘                                                │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Results: Real Numbers

From EdgeQuake production testing:

| Metric                     | Value               |
| -------------------------- | ------------------- |
| Entities per 10k-token doc | 15-25               |
| Relationships per doc      | 10-20               |
| Extraction time            | ~2-10 seconds       |
| Parse success rate         | ~99% (tuple format) |
| Gleaning improvement       | +20-37%             |
| Deduplication rate         | 40-67%              |

Compare to traditional NER:

- 2-3x more entities (with descriptions)
- Relationships included (NER gives you nothing)
- Domain-agnostic (no training needed)

---

## What's Next?

In the next article, we'll explore **Graph Storage Architecture**:

- How PostgreSQL + Apache AGE stores the knowledge graph
- Why one database beats three
- Query patterns for graph traversal

→ [Star EdgeQuake on GitHub](https://github.com/raphaelmansuy/edgequake)

---

## TL;DR

1. **LLMs beat NER** for extraction: richer descriptions, relationships included
2. **Tuple format is robust**: 99% parse success vs 80-90% with JSON
3. **Gleaning finds +20-30% more** entities through iterative re-extraction
4. **Normalization is critical**: 40-67% deduplication for clean graphs
5. **Domain-agnostic**: Works on any text without training

_This is Part 3 of the EdgeQuake Deep Dive series. Follow for more on building production Graph-RAG systems._

**Tags**: #EntityExtraction #NLP #KnowledgeGraphs #LLM #GraphRAG #EdgeQuake #Rust
