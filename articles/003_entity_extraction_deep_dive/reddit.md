# Reddit Post: Entity Extraction Deep Dive

## Subreddits

- r/MachineLearning (technical focus)
- r/LocalLLaMA (LLM practitioners)
- r/LanguageTechnology (NLP community)

---

## r/MachineLearning Post

### Title

[P] LLM-based entity extraction with 99% parse success – lessons from building a Graph-RAG framework

### Post

Hey r/MachineLearning,

We've been working on EdgeQuake, an open-source Graph-RAG framework in Rust, and wanted to share some lessons learned about entity extraction with LLMs.

**The problem we solved:**

Traditional NER gives you labels ("John Smith" → PERSON). LLM-based extraction gives you:

- Rich descriptions ("Lead climate researcher at MIT")
- Explicit relationships (WORKS_AT, COLLABORATES_WITH)
- Domain-agnostic extraction (no training required)

**The JSON trap:**

Initially we asked LLMs to output JSON. 10-20% parse failure rate due to malformed output (missing brackets, unescaped quotes, etc.).

**Our solution: Tuple-delimited format**

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Lead researcher
relation<|#|>SARAH<|#|>MIT<|#|>works_at<|#|>Works at MIT
<|COMPLETE|>
```

Line-by-line parsing with partial recovery. ~99% parse success.

**Key findings:**

| Technique                | Impact                             |
| ------------------------ | ---------------------------------- |
| Tuple format             | 99% vs 80-90% parse success        |
| Gleaning (re-extraction) | +20-30% more entities              |
| Normalization            | 40-67% deduplication               |
| LLM vs NER               | 2-3x more entities + relationships |

**The full pipeline:**

Document → Chunk (600-1200 tokens) → LLM Extract → Parse → Glean → Normalize → Knowledge Graph

Code is open source (Rust): https://github.com/raphaelmansuy/edgequake

Happy to answer questions about the extraction approach or comparisons with other methods (spaCy, fine-tuned NER, structured outputs, etc.).

---

## r/LocalLLaMA Post

### Title

JSON parsing with local LLMs has been painful – here's the tuple format we use instead

### Post

Anyone else struggling with JSON parsing from local LLMs?

We've been building EdgeQuake (Graph-RAG framework) and initially asked the LLM to output JSON for entity extraction. Even with llama.cpp grammar constraints, we hit issues:

- 10-20% malformed output
- Truncation mid-JSON
- Escaping issues in descriptions

**Our solution: Tuple-delimited format**

Instead of:

```json
{ "entities": [{ "name": "SARAH_CHEN", "type": "PERSON" }] }
```

We use:

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Description here
relation<|#|>SARAH<|#|>MIT<|#|>works_at<|#|>...
<|COMPLETE|>
```

Parse line by line. Skip bad lines. Keep good ones.

Works with Ollama, llama.cpp, vLLM—any backend.

**Results:**

- ~99% parse success
- Works with 7B models (tested with Mistral, Llama3)
- Streaming-compatible

Open source (Rust + works with Ollama): https://github.com/raphaelmansuy/edgequake

What extraction formats have worked for you with local LLMs?

---

## Community Guidelines Notes

- r/MachineLearning: [P] tag for project, technical focus, no self-promotion language
- r/LocalLLaMA: Focus on local LLM compatibility, practical problems
- Both: Engage with comments, provide technical details when asked
