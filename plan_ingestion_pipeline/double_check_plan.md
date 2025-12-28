# Double-Check Verification Plan

## Purpose

Exhaustive verification that ALL implementation items from `05-implementation-plan.md` have been implemented in the actual codebase.

## Methodology

1. Cross-reference each task ID against actual source code
2. Verify file existence and location
3. Check function/struct signatures match specifications
4. Confirm test coverage for each component
5. Document any gaps or discrepancies

---

## Phase 1: Core Enhancements + SOTA Prompt System

### Task Checklist

| Task ID | Description                            | Expected File(s)               | Verification Method                                                      |
| ------- | -------------------------------------- | ------------------------------ | ------------------------------------------------------------------------ |
| P1-01   | Add line numbers to TextChunk          | `chunker.rs`                   | Check `start_line`, `end_line` fields, `calculate_line_numbers()`        |
| P1-02   | Add parallel extraction                | `pipeline.rs`                  | Check `extract_parallel()` with semaphore                                |
| P1-03   | Add token tracking to ExtractionResult | `extractor.rs`                 | Check `input_tokens`, `output_tokens`, `extraction_time_ms`              |
| P1-04   | Create prompts module                  | `prompts/mod.rs`               | Verify module exports, delimiters                                        |
| P1-05   | Implement EntityExtractionPrompts      | `prompts/entity_extraction.rs` | Check `system_prompt()`, `user_prompt()`, `continue_extraction_prompt()` |
| P1-06   | Implement TupleParser                  | `prompts/parser.rs`            | Check tuple format parsing with `<\|#\|>` delimiter                      |
| P1-07   | Implement JsonExtractionParser         | `prompts/parser.rs`            | Check JSON format parsing                                                |
| P1-08   | Implement HybridExtractionParser       | `prompts/parser.rs`            | Check auto-detection, fallback logic                                     |
| P1-09   | Implement normalize_entity_name        | `prompts/normalizer.rs`        | Check normalization: prefix removal, possessive, UPPER_SNAKE             |
| P1-10   | Integrate SOTA extractor               | `extractor.rs`                 | Check `SOTAExtractor` struct using chat interface                        |
| P1-11   | Add tests for prompts                  | `prompts/*.rs`                 | Verify unit tests exist                                                  |

### Acceptance Criteria to Verify

- [ ] Line numbers preserved from chunking to extraction
- [ ] Parallel extraction respects concurrency limits
- [ ] Token usage tracked per extraction
- [ ] SOTA prompts match LightRAG format
- [ ] Entity names normalized consistently
- [ ] Hybrid parser detects format automatically
- [ ] Tests cover edge cases

---

## Phase 2: MapReduce & Caching

### Task Checklist

| Task ID | Description                       | Expected File(s)            | Verification Method                                            |
| ------- | --------------------------------- | --------------------------- | -------------------------------------------------------------- |
| P2-01   | Implement MapReduce summarizer    | `summarizer.rs`             | Check `map_reduce_summarize()`, `chunk_descriptions()`         |
| P2-02   | Create LLMCache trait             | `cache.rs`                  | Check trait with `get()`, `set()`, `get_by_chunk()`, `stats()` |
| P2-03   | Implement MemoryLLMCache          | `cache.rs`                  | Check in-memory implementation with indexes                    |
| P2-04   | Implement PostgresLLMCache        | `cache.rs` or storage crate | Check PostgreSQL adapter (if implemented)                      |
| P2-05   | Implement CacheEntry              | `cache.rs`                  | Check struct with TTL, token tracking                          |
| P2-06   | Implement CachedExtractor wrapper | `cache.rs`                  | Check cache-first extraction                                   |
| P2-07   | Add SummarizationPrompts          | `prompts/summarization.rs`  | Check entity/relationship summary prompts                      |
| P2-08   | Add tests for caching             | `cache.rs`                  | Verify cache tests                                             |

### Acceptance Criteria to Verify

- [ ] MapReduce handles large description sets
- [ ] Cache hits skip LLM calls
- [ ] Cache supports TTL expiration
- [ ] Cache stats available for monitoring
- [ ] Summarization prompts follow LightRAG format

---

## Phase 3: Progress & Cost Tracking

### Task Checklist

| Task ID | Description                 | Expected File(s) | Verification Method                                            |
| ------- | --------------------------- | ---------------- | -------------------------------------------------------------- |
| P3-01   | Create progress types       | `progress.rs`    | Check `IngestionProgress`, `StageProgress`, `PipelineStage`    |
| P3-02   | Implement ProgressTracker   | `progress.rs`    | Check thread-safe tracker with `set_stage()`, `update_stage()` |
| P3-03   | Create cost types           | `progress.rs`    | Check `CostBreakdown`, `OperationCost`, `ModelPricing`         |
| P3-04   | Implement CostTracker       | `progress.rs`    | Check `record()`, `snapshot()`, `total_cost()`                 |
| P3-05   | Add default model pricing   | `progress.rs`    | Check `default_model_pricing()` with GPT-4o, Claude, etc.      |
| P3-06   | Create IngestionError type  | `progress.rs`    | Check error with code, stage, recoverable flag                 |
| P3-07   | Create ProgressMessage type | `progress.rs`    | Check message with level, timestamp                            |
| P3-08   | Add tests for progress/cost | `progress.rs`    | Verify unit tests                                              |

### Acceptance Criteria to Verify

- [ ] Progress tracked at stage level
- [ ] Messages recorded in history
- [ ] Errors tracked with context
- [ ] Cost calculated accurately per model
- [ ] Progress can be queried via API (Phase 5)

---

## Phase 4: Lineage & Document Management

### Task Checklist

| Task ID | Description                     | Expected File(s) | Verification Method                                      |
| ------- | ------------------------------- | ---------------- | -------------------------------------------------------- |
| P4-01   | Create lineage types            | `lineage.rs`     | Check `DocumentLineage`, `EntityLineage`, `ChunkLineage` |
| P4-02   | Implement lineage storage       | Storage crate    | Check persistence (if implemented)                       |
| P4-03   | Integrate lineage into pipeline | `pipeline.rs`    | Check lineage building during process                    |
| P4-04   | Implement document suppression  | `documents.rs`   | Check document removal with graph cleanup                |
| P4-05   | Implement cascade delete        | `graph.rs`       | Check orphan entity handling                             |
| P4-06   | Add impact analysis             | API handlers     | Check deletion preview                                   |
| P4-07   | Add tests                       | `lineage.rs`     | Verify lineage tests                                     |

### Acceptance Criteria to Verify

- [ ] Lineage tracks document → chunk → entity/relationship
- [ ] Line numbers preserved in lineage
- [ ] Description history maintained
- [ ] LineageBuilder for easy construction

---

## Phase 5: API & Integration

### Task Checklist

| Task ID | Description                 | Expected File(s) | Verification Method             |
| ------- | --------------------------- | ---------------- | ------------------------------- |
| P5-01   | Add progress endpoints      | API handlers     | Check REST endpoints            |
| P5-02   | Add lineage endpoints       | API handlers     | Check lineage query endpoints   |
| P5-03   | Add cost endpoints          | API handlers     | Check cost retrieval endpoints  |
| P5-04   | Implement WebSocket handler | `ws.rs`          | Check real-time progress events |
| P5-05   | Update OpenAPI spec         | API crate        | Check spec updates              |
| P5-06   | Create E2E tests            | `e2e/*.rs`       | Check integration tests         |
| P5-07   | Update documentation        | `docs/*.md`      | Check docs updated              |

### Acceptance Criteria to Verify

- [ ] All API endpoints documented in OpenAPI
- [ ] WebSocket events work for progress tracking
- [ ] E2E tests cover full ingestion flow

---

## Verification Procedure

For each task:

1. **File Check**: Confirm file exists at expected path
2. **Code Review**: Read implementation and match to spec
3. **Signature Match**: Verify function/struct names match
4. **Test Coverage**: Confirm tests exist and pass
5. **Documentation**: Check doc comments present

## Output

Results will be documented in `verification.md` with:

- ✅ Fully Implemented
- ⚠️ Partially Implemented (with details)
- ❌ Not Implemented (with recommendations)
- 📁 File reference with line numbers
