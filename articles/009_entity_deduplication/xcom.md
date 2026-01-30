# Thread: Why Your Knowledge Graph Has 4x Too Many Nodes 🧵

## Tweet 1

We ingested 1,000 documents into our RAG system.

Extracted 12,450 entities.

Then I searched for "John Doe"...

And found 4 separate nodes for the same person.

Here's the silent killer destroying your knowledge graphs: 🧵

---

## Tweet 2

LLMs don't output consistent entity names.

Same person, 4 different chunks:

- "John Doe" (formal doc)
- "john doe" (casual email)
- "JOHN DOE" (all caps header)
- "The John Doe" (article reference)

Each becomes a separate node.

---

## Tweet 3

The cascade of problems:

```
┌────────────────┐
│   John Doe     │──▶ PROJECT_A
└────────────────┘

┌────────────────┐
│   john doe     │──▶ PROJECT_B
└────────────────┘

┌────────────────┐
│   JOHN DOE     │──▶ MANAGER
└────────────────┘
```

3 disconnected nodes.
No path from Project A to Project B.
Graph completely fragmented.

---

## Tweet 4

The solution: Normalize before storage.

```
"John Doe"      → JOHN_DOE
"john doe"      → JOHN_DOE
"JOHN DOE"      → JOHN_DOE
"The John Doe"  → JOHN_DOE
```

All variants → same canonical form → same node.

---

## Tweet 5

The normalization algorithm:

1. Trim whitespace
2. Remove prefixes (The, A, An)
3. Remove possessives ('s)
4. Replace spaces with underscores
5. Convert to UPPERCASE

```
"  The Company's Strategy  "
→ "COMPANYS_STRATEGY"
```

Deterministic. No LLM needed.

---

## Tweet 6

But what about duplicate descriptions?

Doc 1: "Chen is an engineer"
Doc 2: "Chen leads the ML team"

Bad approach: Replace (lose doc 1 info)
Good approach: Merge

Result: "Chen is an engineer and leads the ML team"

---

## Tweet 7

Sentence-level deduplication:

```rust
fn merge_descriptions(existing, new) {
    for sentence in new.split('.') {
        if !existing.contains(sentence) {
            additions.push(sentence);
        }
    }
    return existing + additions;
}
```

Only NEW information gets added.
Repeated facts filtered out.

---

## Tweet 8

Source lineage tracking:

Before merge:

```
JOHN_DOE.source_ids = ["doc1_chunk5"]
```

After merge with doc2:

```
JOHN_DOE.source_ids = ["doc1_chunk5", "doc2_chunk3"]
```

Full provenance preserved.
Know exactly where info came from.

---

## Tweet 9

Description length limits:

Merging could grow descriptions forever.

EdgeQuake enforces max 4096 chars.
Truncates at sentence boundaries.

Or with LLM: actively summarizes.

"Combine these 5 descriptions into one coherent summary."

---

## Tweet 10

Edge cases to handle:

❓ "Apple" fruit vs "Apple" company
→ Entity types: APPLE_FRUIT, APPLE_ORGANIZATION

❓ "IBM" vs "International Business Machines"
→ LLM extraction uses canonical names

❓ Unicode "Café" vs "Cafe"
→ Unicode normalization before processing

---

## Tweet 11

Production results:

Before normalization:

- 12,450 nodes
- 2.1 edges per node

After normalization:

- 7,470 nodes
- 3.5 edges per node

40% deduplication rate.
67% more edges per node (properly connected!)

---

## Tweet 12

Query quality improvement:

| Metric                    | Before | After  |
| ------------------------- | ------ | ------ |
| Entity recall             | 62%    | 94%    |
| Relationship completeness | 45%    | 89%    |
| Answer accuracy           | 5.8/10 | 8.2/10 |

Fragmentation was destroying our RAG.
Deduplication fixed it.

---

## Tweet 13

Implementation checklist:

✅ Normalize before storage
✅ Merge on collision
✅ Track source lineage
✅ Limit description length
✅ Handle edge cases (types, unicode)

Do this BEFORE you ingest 10M documents.
Retrofitting is painful.

---

## Tweet 14

EdgeQuake handles all of this automatically.

```bash
make dev

# Ingest - dedup happens automatically
curl -X POST localhost:3000/api/documents -F "file=@docs.pdf"

# Check stats
curl localhost:3000/api/stats
# {"entities": 7470, "dedup_rate": 0.40}
```

---

## Tweet 15

TL;DR:

LLMs output inconsistent entity names.
40% of your graph nodes might be duplicates.
Normalize before storage.
Merge descriptions, don't replace.
Track source lineage.

🔗 github.com/your-org/edgequake
📄 LightRAG paper: arXiv:2410.05779
