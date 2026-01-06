# OODA Loop 3 - Orient

## Analysis of Test Results

### Recall Assessment: ✅ GOOD
- Entity embeddings now stored in vector storage
- Sources count per query: 47-65 (high recall)
- All document types being searched

### Precision Assessment: ⚠️ NEEDS IMPROVEMENT

#### Observed Issues

1. **Document Confusion**
   - Query: "Prix du Peugeot 2008 ENVY"
   - Retrieved: 3008, 208, 5008 chunks (higher scores)
   - Expected: 2008 chunks with ENVY price
   - Root cause: Numeric model names (2008, 3008, 208) have similar embeddings

2. **Entity Name Overlap**
   - "PEUGEOT 2008" entity embedding similar to "PEUGEOT 3008"
   - Semantic similarity high for numeric model designations

3. **Chunk Granularity**
   - Some chunks may be too large or too small
   - Price info may be in a chunk that doesn't rank high

### Precision Formula Analysis

```
Precision = Relevant Docs Retrieved / Total Docs Retrieved

Current state:
- Query about 2008 ENVY price
- Top 3 chunks: 3008, 208, 5008 (0 relevant)
- Precision for this query: 0%

Root cause: Embedding similarity for numeric car models is too high
```

### Potential Fixes

1. **Boost Exact Match Terms**
   - If query contains "2008", boost chunks containing "2008"
   - Keyword matching + semantic similarity hybrid

2. **Entity Deduplication/Disambiguation**
   - "PEUGEOT 2008" and "PEUGEOT 3008" need disambiguation
   - Add model variant to entity names

3. **Document-Level Filtering**
   - Filter by document_id after entity match
   - If searching for 2008, only return chunks from 2008 document

4. **Rerank with Query Keywords**
   - After vector search, boost results containing query keywords

