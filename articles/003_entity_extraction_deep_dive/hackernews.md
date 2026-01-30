# HackerNews Post: Entity Extraction Deep Dive

## Title

Show HN: We ditched JSON for LLM entity extraction – here's what we learned

## Post

We've been building EdgeQuake, an open-source Graph-RAG framework in Rust. The core challenge: extracting entities and relationships from documents using LLMs.

**The JSON problem**

Our first implementation asked LLMs to output JSON:

```json
{
  "entities": [{ "name": "SARAH_CHEN", "type": "PERSON", "description": "..." }]
}
```

10-20% failure rate. Missing brackets, unescaped quotes, truncated output. One malformed character and you lose the entire extraction.

**The tuple solution**

We switched to a tuple-delimited format (inspired by LightRAG):

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Lead researcher
relation<|#|>SARAH_CHEN<|#|>MIT<|#|>works_at<|#|>Sarah works at MIT
<|COMPLETE|>
```

Parse success: ~99%. Bad lines get skipped, good ones get kept.

**Key learnings:**

1. **Tuple > JSON for LLM output** – Line-by-line parsing with partial recovery beats all-or-nothing JSON parsing

2. **Gleaning works** – Re-prompting with "find what you missed" gets 20-30% more entities

3. **Normalization is critical** – Without it, "John Doe", "john doe", and "JOHN DOE" become 3 separate nodes. We see 40-67% deduplication after normalization.

4. **LLMs beat NER for richness** – Traditional NER gives you labels. LLMs give you descriptions + relationships. 2-3x more knowledge extracted.

The full extraction pipeline: Document → Chunk → LLM Extract (tuples) → Parse → Glean → Normalize → Graph

Open source (Rust): https://github.com/raphaelmansuy/edgequake

Curious to hear if others have hit similar JSON parsing issues with LLMs, and what alternatives you've tried.

---

## Expected Discussion Points

- JSON vs structured output debate
- Pydantic/structured outputs as alternative
- NER comparison (spaCy, etc.)
- Token efficiency of tuple format
- Gleaning effectiveness in practice
