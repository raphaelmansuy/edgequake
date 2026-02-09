# Deep Dive: Chunking Strategies

> **How EdgeQuake Splits Documents for Processing**

This guide explains document chunking in EdgeQuake, including strategies, configuration, and optimization.

---

## Why Chunking Matters

```
┌─────────────────────────────────────────────────────────────────┐
│                 THE CHUNKING PROBLEM                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Documents are too large for:                                   │
│  • LLM context windows (limited tokens)                         │
│  • Embedding models (max ~8K tokens)                            │
│  • Precise retrieval (large docs = low relevance)               │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ 50-page PDF (25,000 tokens)                              │   │
│  │                                                            
│  │ ❌ Can't embed whole document                              
│  │ ❌ LLM can't process all at once                          
│  │ ❌ If query matches page 3, all 50 pages retrieved        │
│  └──────────────────────────────────────────────────────────┘   │
│                            ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Solution: Split into chunks                              │   │
│  │                                                          │   │
│  │ [Chunk 1: 500 tokens] [Chunk 2: 500 tokens] ...          │   │
│  │                                                          │   │
│  │ ✅ Each chunk embeddable                                   
│  │ ✅ Precise retrieval (only relevant chunks)               
│  │ ✅ LLM can process multiple chunks in context             
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Default Configuration

EdgeQuake's default chunking configuration:

```rust
ChunkerConfig {
    chunk_size: 1200,        // Target tokens per chunk
    chunk_overlap: 100,      // Overlap between chunks
    min_chunk_size: 100,     // Minimum chunk size
    separators: [
        "\n\n",              // Paragraph breaks (highest priority)
        "\n",                // Line breaks
        ". ",                // Sentences
        "! ",
        "? ",
        "; ",
        ", ",
        " ",                 // Words (lowest priority)
    ],
    preserve_sentences: true,
}
```

**Why These Defaults**:

- **1200 tokens**: Fits well in LLM context (~5 chunks in 8K context)
- **100 token overlap**: 8% overlap captures boundary entities
- **Separator priority**: Preserves semantic structure

---

## Overlap: Why It Matters

```
┌─────────────────────────────────────────────────────────────────┐
│                 OVERLAP PREVENTS INFORMATION LOSS               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  WITHOUT OVERLAP (Bad):                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ "Dr. Smith works at Microsoft. He developed the Azure"   │   │
│  │                              ↑                           │   │
│  │                          CUT HERE                        │   │
│  │ "platform with his team at the Seattle campus."          │   │
│  │                                                          │   │
│  │ Problem: "He" in chunk 2 has no context                  │   │
│  │ Problem: "Azure platform" split across chunks            │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  WITH OVERLAP (Good):                                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Chunk 1: "Dr. Smith works at Microsoft. He developed"    │   │
│  │ Chunk 2: "He developed the Azure platform with his team" │   │
│  │                                                          │   │
│  │ ✅ "He" has context from overlap                         
│  │ ✅ "Azure platform" appears complete in chunk 2          
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Recommended: 8-15% overlap (100-180 tokens for 1200 chunk)     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Chunking Strategies

EdgeQuake supports three built-in chunking strategies:

### 1. Token-Based Chunking (Default)

Splits text by token count, respecting separator hierarchy.

```
┌─────────────────────────────────────────────────────────────────┐
│                 TOKEN-BASED CHUNKING                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Input: "Paragraph 1...\n\nParagraph 2...\n\nParagraph 3..."    │
│                                                                 │
│  Algorithm:                                                     │
│  1. Try to split on "\n\n" (paragraph)                          │
│  2. If chunk too large, try "\n" (line)                         │
│  3. If still too large, try ". " (sentence)                     │
│  4. Continue down separator list                                │
│  5. Last resort: split on " " (word)                            │
│                                                                 │
│  Result: Clean chunks at natural boundaries                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Best For**: General documents, articles, reports

### 2. Sentence Boundary Chunking

Never splits mid-sentence, accumulates complete sentences.

```
┌─────────────────────────────────────────────────────────────────┐
│                 SENTENCE BOUNDARY CHUNKING                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Input: "Sentence 1. Sentence 2. Sentence 3. Sentence 4."       │
│                                                                 │
│  Algorithm:                                                     │
│  1. Split text into sentences                                   │
│  2. Accumulate sentences until target size                      │
│  3. Create chunk                                                │
│  4. Overlap: Carry last N sentences to next chunk               │
│                                                                 │
│  Guarantees:                                                    │
│  • Every sentence is complete                                   │
│  • No orphaned pronouns                                         │
│  • Better entity extraction context                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Best For**: Legal documents, research papers, any text where sentence integrity matters

### 3. Character-Based Chunking

Splits on a specific character (e.g., newline).

```
┌─────────────────────────────────────────────────────────────────┐
│                 CHARACTER-BASED CHUNKING                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Use Cases:                                                     │
│  • Pre-split content (CSV, TSV)                                 │
│  • Log files (one entry per line)                               │
│  • Markdown headers (split on "## ")                            │
│                                                                 │
│  Configuration:                                                 │
│  {                                                              │
│    "split_by_character": "\n",                                  │
│    "split_by_character_only": true                              │
│  }                                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Best For**: Log files, structured data, pre-segmented content

---

## ChunkingStrategy Trait

EdgeQuake's chunking is extensible via the `ChunkingStrategy` trait:

```rust
#[async_trait]
pub trait ChunkingStrategy: Send + Sync {
    /// Chunk the given text content into smaller pieces.
    async fn chunk(&self, content: &str, config: &ChunkerConfig)
        -> Result<Vec<ChunkResult>>;

    /// Get the name of this chunking strategy.
    fn name(&self) -> &str;
}

/// Result of a custom chunking operation.
pub struct ChunkResult {
    pub content: String,
    pub tokens: usize,
    pub chunk_order_index: usize,
}
```

### Implementing Custom Chunking

```rust
/// Markdown-aware chunking (splits on headers)
pub struct MarkdownChunking;

#[async_trait]
impl ChunkingStrategy for MarkdownChunking {
    async fn chunk(&self, content: &str, _config: &ChunkerConfig)
        -> Result<Vec<ChunkResult>> {
        // Split on markdown headers
        let sections: Vec<&str> = content
            .split("\n## ")
            .collect();

        Ok(sections
            .into_iter()
            .enumerate()
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(idx, s)| ChunkResult {
                content: s.to_string(),
                tokens: s.len() / 4,  // Rough estimate
                chunk_order_index: idx,
            })
            .collect())
    }

    fn name(&self) -> &str {
        "markdown"
    }
}
```

---

## Size Tradeoffs

```
┌─────────────────────────────────────────────────────────────────┐
│                 CHUNK SIZE TRADEOFFS                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  SMALL CHUNKS (256-512 tokens):                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ✅ More precise retrieval                                 
│  │ ✅ Lower per-chunk embedding cost                         
│  │ ✅ Faster embedding generation                            
│  │ ❌ More LLM extraction calls                              
│  │ ❌ Less context per chunk                                 
│  │ ❌ Entity relationships may span chunks                   
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  LARGE CHUNKS (1024-2048 tokens):                               │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ✅ Better context for entity extraction                 
│  │ ✅ Fewer LLM calls                                        
│  │ ✅ Relationships captured within chunk                    
│  │ ❌ Lower retrieval precision                              
│  │ ❌ Higher per-chunk embedding cost                        
│  │ ❌ May hit embedding model limits                         
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Recommendation: Start with 1200 tokens (default)               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Chunk Size Guidelines

| Document Type   | Recommended Size | Overlap |
| --------------- | ---------------- | ------- |
| Research papers | 1200-1500        | 100-150 |
| News articles   | 800-1000         | 80-100  |
| Legal contracts | 1500-2000        | 150-200 |
| Technical docs  | 1000-1200        | 100     |
| Chat logs       | 500-800          | 50-80   |
| Code files      | 800-1000         | 100     |

---

## TextChunk Structure

Each chunk includes rich metadata:

```rust
pub struct TextChunk {
    /// Unique identifier for the chunk.
    pub id: String,

    /// The chunk text content.
    pub content: String,

    /// Index of this chunk in the document.
    pub index: usize,

    /// Character offset from the start of the document.
    pub start_offset: usize,

    /// Character offset to the end of the chunk.
    pub end_offset: usize,

    /// Starting line number (1-based) in the original document.
    pub start_line: usize,

    /// Ending line number (1-based, inclusive).
    pub end_line: usize,

    /// Approximate token count.
    pub token_count: usize,

    /// Chunk embedding (populated after embedding stage).
    pub embedding: Option<Vec<f32>>,
}
```

**Why Line Numbers**:

- Citations in query responses ("See document.pdf, lines 45-52")
- Debugging entity extraction
- Source verification

---

## Configuration

### Via Environment Variables

```bash
# Coming soon - currently configured via API
```

### Via API

```bash
# Create workspace with custom chunk settings
curl -X POST http://localhost:8080/api/v1/tenants/default/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "research",
    "chunking_config": {
      "chunk_size": 1500,
      "chunk_overlap": 150,
      "min_chunk_size": 100,
      "preserve_sentences": true
    }
  }'
```

### Via PipelineConfig

```rust
let config = PipelineConfig {
    chunker: ChunkerConfig {
        chunk_size: 1500,
        chunk_overlap: 150,
        min_chunk_size: 100,
        separators: vec![
            "\n\n".to_string(),
            "\n".to_string(),
            ". ".to_string(),
        ],
        preserve_sentences: true,
        split_by_character: None,
        split_by_character_only: false,
    },
    ..Default::default()
};
```

---

## Chunking Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                 CHUNKING PIPELINE                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document Text                                                  │
│       ↓                                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 1. PREPROCESSING                                           │ │
│  │    • Remove excessive whitespace                           │ │
│  │    • Normalize line endings                                │ │
│  │    • Handle Unicode normalization                          │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ↓                                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 2. SEPARATOR DETECTION                                     │ │
│  │    • Find paragraph breaks ("\n\n")                        │ │
│  │    • Find line breaks ("\n")                               │ │
│  │    • Find sentence endings (". ", "! ", "? ")              │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ↓                                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 3. SPLIT ON BEST SEPARATOR                                 │ │
│  │    • Try highest priority separator first                  │ │
│  │    • If chunks too large, try next separator               │ │
│  │    • Continue until target size reached                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ↓                                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 4. OVERLAP CREATION                                        │ │
│  │    • Copy end of chunk N to start of chunk N+1             │ │
│  │    • Ensure overlap respects sentence boundaries           │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ↓                                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 5. METADATA ENRICHMENT                                     │ │
│  │    • Calculate line numbers                                │ │
│  │    • Assign chunk indices                                  │ │
│  │    • Generate chunk IDs                                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ↓                                                         │
│  Vec<TextChunk>                                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Token Estimation

EdgeQuake uses a simple heuristic for token estimation:

```rust
/// Estimate token count (rough approximation: 1 token ≈ 4 chars).
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}
```

**Why Estimate**:

- Actual tokenization requires model-specific tokenizer
- 4 chars/token is accurate for English (within 10%)
- Much faster than calling tokenizer API

**Accuracy**:
| Language | Actual Tokens/Char | Estimate Error |
|----------|-------------------|----------------|
| English | ~4.0 | <10% |
| German | ~3.5 | ~15% |
| Chinese | ~1.5 | ~60% |
| Code | ~5.0 | ~20% |

For precise control, override with custom ChunkingStrategy.

---

## Performance Optimization

### Parallel Chunking

EdgeQuake chunks documents in parallel:

```rust
// Documents are chunked in parallel
let chunks: Vec<TextChunk> = documents
    .par_iter()  // Rayon parallel iterator
    .flat_map(|doc| chunker.chunk(&doc.content))
    .collect();
```

### Streaming Large Documents

For very large documents (>100MB), consider streaming:

```bash
# Split file before upload
split -b 10m large_document.txt part_

# Upload parts
for f in part_*; do
  curl -X POST http://localhost:8080/api/v1/documents/upload \
    -F "file=@$f"
done
```

---

## Troubleshooting

### Chunks Too Small

```
Warning: Many chunks below min_chunk_size
```

**Cause**: Document has many short paragraphs.

**Solution**: Reduce `min_chunk_size` or increase `chunk_size`:

```json
{ "chunk_size": 2000, "min_chunk_size": 50 }
```

### Chunks Too Large

```
Error: Chunk exceeds embedding model limit
```

**Cause**: No suitable separators found.

**Solution**: Add more separators:

```json
{
  "separators": ["\n\n", "\n", ". ", "! ", "? ", "; ", ", ", " "]
}
```

### Lost Context

```
Query returns incomplete answers
```

**Cause**: Overlap too small.

**Solution**: Increase overlap:

```json
{ "chunk_overlap": 200 }
```

---

## Best Practices

1. **Start with Defaults**: 1200 tokens, 100 overlap works for most cases
2. **Match Content Type**: Use sentence boundary for legal, token for general
3. **Test Retrieval**: Query sample documents to validate chunk quality
4. **Monitor Chunk Stats**: Check `avg_chunk_size` in pipeline stats
5. **Consider Domain**: Technical content may need larger chunks
6. **Preserve Structure**: Use custom separators for structured docs

---

## See Also

- [Embedding Models](./embedding-models.md) - How chunks are embedded
- [Entity Extraction](./entity-extraction.md) - How entities are extracted from chunks
- [LightRAG Algorithm](./lightrag-algorithm.md) - Overall pipeline design
