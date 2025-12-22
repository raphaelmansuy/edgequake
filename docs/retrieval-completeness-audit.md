# EdgeQuake Retrieval Completeness Audit

**Date:** 2025-01-22  
**Auditor:** AI Assistant  
**Scope:** Complete comparison of EdgeQuake vs LightRAG retrieval algorithms

---

## Executive Summary

After deep analysis of:

1. `docs_retro/05-algorithms.md` - LightRAG algorithm specification
2. `lightrag/operate.py` - Python implementation (~5000 lines)
3. `edgequake/crates/` - Rust implementation

**Status:** ✅ **CORE ALGORITHMS COMPLETE** | ⚠️ **ADVANCED FEATURES MISSING**

**Already Implemented (Recent Session):**

- ✅ Entity vector search (Local mode)
- ✅ Relationship vector search (Global mode)
- ✅ Type-based vector filtering (chunk/entity/relationship)
- ✅ Hybrid mode with basic merging
- ✅ Mix mode with weighted combination
- ✅ Round-robin entity/relationship merging

**Missing Features (Found in LightRAG):**

- ❌ Keyword extraction (high-level + low-level)
- ❌ Token-based truncation for LLM efficiency
- ❌ Chunk reranking by frequency/vector similarity
- ❌ Entity degree/rank-based sorting
- ❌ Related chunk retrieval from entities
- ❌ Conversational history support
- ❌ Response type customization
- ❌ Only-context and only-prompt modes

---

## Detailed Gap Analysis

### 1. Keyword Extraction ❌ CRITICAL MISSING

**LightRAG Implementation:**

```python
async def extract_keywords_only(text, param, global_config, hashing_kv):
    # Call LLM to extract high-level and low-level keywords
    kw_prompt = PROMPTS["keywords_extraction"].format(
        query=text,
        examples=examples,
        language=language,
    )
    response = await llm_func(kw_prompt)
    keywords_data = json.loads(response)
    return keywords_data["high_level_keywords"], keywords_data["low_level_keywords"]
```

**Purpose:**

- **High-level keywords:** Abstract concepts, themes, topics → used for Global mode
- **Low-level keywords:** Specific entities, technical terms → used for Local mode
- Allows bypass of keyword extraction when provided by user
- Cached for performance

**EdgeQuake:**

- ❌ **Missing entirely**
- Queries are used directly as-is for vector search
- No distinction between high/low-level concepts

**Impact:**

- **Severe:** Local/Global modes less effective without proper keyword targeting
- Users can't bypass LLM for faster queries
- No query preprocessing/optimization

---

### 2. Token-Based Truncation ❌ CRITICAL MISSING

**LightRAG Implementation:**

```python
async def _apply_token_truncation(search_result, query_param, global_config):
    tokenizer = global_config["tokenizer"]

    # Truncate entities by token count
    entity_texts = [format_entity(e) for e in entities]
    entities_truncated = truncate_list_by_token_size(
        entity_texts,
        query_param.max_entity_tokens
    )

    # Truncate relations by token count
    relation_texts = [format_relation(r) for r in relations]
    relations_truncated = truncate_list_by_token_size(
        relation_texts,
        query_param.max_relation_tokens
    )

    # Ensure total doesn't exceed max_total_tokens
    total_tokens = count_tokens(entities + relations + chunks)
    if total_tokens > query_param.max_total_tokens:
        # Proportionally reduce
        ...
```

**Parameters:**

- `max_entity_tokens`: Max tokens for entity context (default: 8000)
- `max_relation_tokens`: Max tokens for relationship context (default: 8000)
- `max_total_tokens`: Max tokens for entire context (default: 16000)

**EdgeQuake:**

- ❌ **Missing entirely**
- Uses fixed counts: `max_chunks`, `max_entities`
- No token-aware truncation
- Risk of context window overflow

**Impact:**

- **Severe:** Can exceed LLM context limits
- Inefficient use of context budget
- No fine-grained control over entity vs relation balance

---

### 3. Chunk Retrieval from Entities ❌ HIGH PRIORITY

**LightRAG Implementation:**

```python
async def _find_related_text_unit_from_entities(
    node_datas,
    query_param,
    text_chunks_db,
    knowledge_graph_inst,
    query,
    chunks_vdb,
    chunk_tracking,
    query_embedding
):
    # Two methods:

    # Method 1: WEIGHT - Frequency-based polling
    chunk_freq = Counter()  # chunk_id -> frequency
    for entity in node_datas:
        source_ids = entity["source_id"].split("|")
        for chunk_id in source_ids:
            chunk_freq[chunk_id] += 1

    # Use linear gradient weighted polling
    selected = pick_by_weighted_polling(
        chunk_freq,
        query_param.related_chunk_number
    )

    # Method 2: VECTOR - Similarity-based
    all_chunks = [text_chunks_db.get(cid) for cid in chunk_freq.keys()]
    selected = pick_by_vector_similarity(
        all_chunks,
        query_embedding,
        query_param.chunk_top_k
    )

    return selected_chunks
```

**EdgeQuake:**

- ❌ **Missing entirely**
- Local mode retrieves entities but doesn't get their source chunks
- No chunk frequency tracking
- No vector-based chunk reranking

**Impact:**

- **High:** Local mode returns entities without supporting evidence
- Users don't see original text that mentioned entities
- Less verifiable responses

---

### 4. Entity Degree/Rank Sorting ❌ MEDIUM PRIORITY

**LightRAG Implementation:**

```python
async def _get_node_data(query, knowledge_graph_inst, entities_vdb, query_param):
    # Vector search for entities
    results = await entities_vdb.query(query, top_k=query_param.top_k)
    node_ids = [r["entity_name"] for r in results]

    # Get node degrees (graph centrality)
    node_degrees = await knowledge_graph_inst.node_degrees_batch(node_ids)

    # Attach degree/rank to each entity
    node_datas = [
        {
            **node,
            "entity_name": entity_name,
            "rank": degree,  # Used for sorting relationships
        }
        for entity_name, node, degree in zip(node_ids, nodes, node_degrees)
    ]

    # Relationships sorted by: rank (degree) + weight
    edges = sorted(edges, key=lambda x: (x["rank"], x["weight"]), reverse=True)

    return node_datas, edges
```

**Purpose:**

- Prioritize central/important entities in graph
- Sort relationships by importance (degree + edge weight)
- Helps select most relevant connections

**EdgeQuake:**

- ✅ Has `node_degree()` method
- ❌ **Not used in query strategies**
- ❌ No relationship sorting by importance

**Impact:**

- **Medium:** Results not optimally ordered
- Less relevant entities/relationships may appear first
- Suboptimal context for LLM

---

### 5. Relationship Chunk Retrieval ❌ MEDIUM PRIORITY

**LightRAG Implementation:**

```python
async def _find_related_text_unit_from_relations(
    edge_datas,
    query_param,
    text_chunks_db,
    entity_chunks,
    query,
    chunks_vdb,
    chunk_tracking,
    query_embedding
):
    # Collect all chunks from relationships
    for edge in edge_datas:
        source_ids = edge["source_id"].split("|")
        for chunk_id in source_ids:
            chunk_freq[chunk_id] += 1

    # Method 1: WEIGHT - Linear gradient polling
    selected = pick_by_weighted_polling(chunk_freq, query_param.related_chunk_number)

    # Method 2: VECTOR - Rerank by similarity
    selected = pick_by_vector_similarity(all_chunks, query_embedding, query_param.chunk_top_k)

    return selected_chunks
```

**EdgeQuake:**

- ❌ **Missing entirely**
- Global mode returns relationships but no source chunks
- No evidence for relationship claims

**Impact:**

- **Medium:** Global mode lacks supporting text
- Less verifiable relationship information
- Users can't verify where relationships came from

---

### 6. Chunk Frequency Tracking ❌ MEDIUM PRIORITY

**LightRAG Implementation:**

```python
# Track chunks across all retrieval sources
chunk_tracking = {}  # chunk_id -> {source, frequency, order}

# From local entities
for entity in local_entities:
    for chunk_id in entity["source_id"].split("|"):
        if chunk_id not in chunk_tracking:
            chunk_tracking[chunk_id] = {"source": "E", "frequency": 0, "order": 0}
        chunk_tracking[chunk_id]["frequency"] += 1

# From global relations
for relation in global_relations:
    for chunk_id in relation["source_id"].split("|"):
        if chunk_id not in chunk_tracking:
            chunk_tracking[chunk_id] = {"source": "R", "frequency": 0, "order": 0}
        chunk_tracking[chunk_id]["frequency"] += 1

# From vector search
for i, chunk in enumerate(vector_chunks):
    chunk_id = chunk.get("chunk_id")
    if chunk_id:
        chunk_tracking[chunk_id] = {"source": "C", "frequency": 1, "order": i+1}
```

**Purpose:**

- Track which chunks appear in multiple retrieval paths
- Prioritize chunks mentioned by multiple entities/relationships
- Log chunk sources for debugging: E (entity), R (relationship), C (vector)

**EdgeQuake:**

- ❌ **Missing entirely**
- No cross-source chunk tracking
- No frequency-based prioritization

**Impact:**

- **Medium:** Misses chunks that are highly relevant (mentioned multiple times)
- No visibility into retrieval process
- Can't implement weighted polling

---

### 7. Conversation History Support ❌ LOW PRIORITY

**LightRAG Implementation:**

```python
# In QueryParam
class QueryParam:
    conversation_history: List[Dict[str, str]] = []  # [{"role": "user", "content": "..."}, ...]

# In kg_query
response = await use_model_func(
    user_query,
    system_prompt=sys_prompt,
    history_messages=query_param.conversation_history,  # Pass conversation history
    enable_cot=True,
    stream=query_param.stream,
)
```

**Purpose:**

- Support multi-turn conversations
- Context from previous exchanges
- Better follow-up question handling

**EdgeQuake:**

- ❌ **Missing entirely**
- Each query is isolated
- No conversation context

**Impact:**

- **Low:** Can be added as enhancement
- Currently supports single-turn Q&A
- Users can't ask follow-up questions effectively

---

### 8. Response Type Customization ❌ LOW PRIORITY

**LightRAG Implementation:**

```python
# In QueryParam
response_type: str = "Multiple Paragraphs"  # or "Single Paragraph", "Bullet Points", etc.

# In prompt
sys_prompt = PROMPTS["rag_response"].format(
    response_type=response_type,  # Controls output format
    user_prompt=user_prompt,
    context_data=context_result.context,
)
```

**Purpose:**

- Control LLM output format
- Match user preferences (paragraphs, bullets, tables, etc.)
- Consistent formatting

**EdgeQuake:**

- ❌ **Missing entirely**
- Fixed response format
- No user control

**Impact:**

- **Low:** Nice-to-have feature
- Can be added to prompt template
- Doesn't affect retrieval quality

---

### 9. Only-Context and Only-Prompt Modes ❌ LOW PRIORITY

**LightRAG Implementation:**

```python
# In QueryParam
only_need_context: bool = False  # Return raw context without LLM response
only_need_prompt: bool = False   # Return formatted prompt without calling LLM

# In kg_query
if query_param.only_need_context and not query_param.only_need_prompt:
    return QueryResult(content=context_result.context, raw_data=context_result.raw_data)

if query_param.only_need_prompt:
    prompt_content = "\n\n".join([sys_prompt, "---User Query---", user_query])
    return QueryResult(content=prompt_content, raw_data=context_result.raw_data)
```

**Purpose:**

- Debug retrieval without LLM cost
- Inspect generated prompts
- Test context quality separately

**EdgeQuake:**

- ✅ Has `context_only` mode
- ❌ **Missing** `only_need_prompt` mode

**Impact:**

- **Low:** Debugging feature
- Can be added easily
- Not critical for production

---

### 10. Reranking Support ❌ LOW PRIORITY

**LightRAG Implementation:**

```python
# In QueryParam
enable_rerank: bool = False
min_rerank_score: float = 0.0

# In _apply_token_truncation
if query_param.enable_rerank:
    # Rerank entities/relations/chunks by relevance
    reranked = rerank_by_query_similarity(items, query_embedding)
    final_items = [item for item in reranked if item.score >= query_param.min_rerank_score]
```

**Purpose:**

- Improve relevance after initial retrieval
- Filter low-quality results
- Better LLM input

**EdgeQuake:**

- ❌ **Missing entirely**
- No reranking step
- Raw vector search results used

**Impact:**

- **Low:** Advanced optimization
- Can improve quality but not critical
- Adds latency

---

## Implementation Priority Matrix

| Feature                       | Priority    | Effort | Impact | Status     |
| ----------------------------- | ----------- | ------ | ------ | ---------- |
| Keyword Extraction            | 🔴 CRITICAL | High   | Severe | ❌ Missing |
| Token-Based Truncation        | 🔴 CRITICAL | Medium | Severe | ❌ Missing |
| Chunk Retrieval from Entities | 🟡 HIGH     | Medium | High   | ❌ Missing |
| Entity Degree Sorting         | 🟢 MEDIUM   | Low    | Medium | ❌ Missing |
| Relationship Chunk Retrieval  | 🟢 MEDIUM   | Medium | Medium | ❌ Missing |
| Chunk Frequency Tracking      | 🟢 MEDIUM   | Medium | Medium | ❌ Missing |
| Conversation History          | 🔵 LOW      | Low    | Low    | ❌ Missing |
| Response Type                 | 🔵 LOW      | Low    | Low    | ❌ Missing |
| Only-Prompt Mode              | 🔵 LOW      | Low    | Low    | ❌ Missing |
| Reranking                     | 🔵 LOW      | High   | Low    | ❌ Missing |

---

## Recommended Implementation Order

### Phase 1: Critical Features (1-2 weeks)

1. **Keyword Extraction** (3-4 days)

   - Add LLM prompt for keyword extraction
   - Support user-provided keywords
   - Implement caching
   - Update query strategies to use keywords

2. **Token-Based Truncation** (2-3 days)
   - Add tokenizer to EdgeQuake config
   - Implement `truncate_by_tokens()` function
   - Add `max_entity_tokens`, `max_relation_tokens`, `max_total_tokens`
   - Apply in all query strategies

### Phase 2: High Priority Features (1 week)

3. **Chunk Retrieval from Entities** (3-4 days)

   - Implement `find_related_chunks_from_entities()`
   - Add frequency-based weighted polling
   - Add vector similarity reranking
   - Integrate into Local mode

4. **Entity Degree Sorting** (1-2 days)
   - Use existing `node_degree()` in strategies
   - Sort entities by degree
   - Sort relationships by degree + weight

### Phase 3: Medium Priority Features (1 week)

5. **Relationship Chunk Retrieval** (2-3 days)

   - Implement `find_related_chunks_from_relationships()`
   - Integrate into Global mode
   - Add frequency tracking

6. **Chunk Frequency Tracking** (2-3 days)
   - Add tracking structure
   - Track across all retrieval paths
   - Use for prioritization

### Phase 4: Polish & Enhancement (1 week)

7. **Conversation History** (2 days)
8. **Response Type** (1 day)
9. **Only-Prompt Mode** (1 day)
10. **Reranking** (2-3 days)

---

## Testing Requirements

### Unit Tests (Per Feature)

- Keyword extraction with various query types
- Token truncation edge cases
- Chunk retrieval algorithms (WEIGHT vs VECTOR)
- Degree sorting correctness
- Frequency tracking accuracy

### Integration Tests

- End-to-end query with all features enabled
- Compare output quality vs LightRAG
- Performance benchmarks

### E2E Tests (Must Add)

- Test with large knowledge graphs (1M+ entities)
- Multi-turn conversation flows
- Token limit stress tests
- All query modes with advanced features

---

## Success Criteria

### Functional Completeness

- ✅ All 10 missing features implemented
- ✅ Unit tests pass (100% coverage on new code)
- ✅ Integration tests pass
- ✅ E2E tests pass

### Quality Metrics

- ✅ Retrieval quality matches LightRAG (manual evaluation)
- ✅ Token usage stays within limits
- ✅ Response relevance improves vs baseline

### Performance

- ✅ Keyword extraction adds <200ms latency
- ✅ Token truncation adds <50ms latency
- ✅ Chunk retrieval adds <300ms latency
- ✅ Total query time <2s for typical queries

---

## Next Steps

1. **Review this audit** with team
2. **Prioritize features** based on project goals
3. **Create detailed specs** for Phase 1 features
4. **Start implementation** with keyword extraction
5. **Iterative testing** after each feature
6. **Document** all new APIs and configs

---

## Conclusion

EdgeQuake has **solid foundations** with core retrieval algorithms implemented correctly. However, **10 important features from LightRAG are missing**. The most critical are:

1. **Keyword Extraction** - Essential for effective Local/Global mode targeting
2. **Token-Based Truncation** - Prevents context overflow, enables fine-grained control
3. **Chunk Retrieval from Entities** - Provides evidence and improves verifiability

Implementing these 3 critical features should be **highest priority**. The remaining 7 features can be added incrementally based on user feedback and specific use case needs.

**Estimated Timeline:**

- Critical features: 1-2 weeks
- High priority: 1 week
- Medium priority: 1 week
- Polish: 1 week
- **Total: 4-5 weeks** for complete feature parity with LightRAG
