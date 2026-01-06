# Search Quality Non-Regression Test Suite

This test suite ensures the EdgeQuake search quality remains high, particularly for French automotive queries.

## 📋 Overview

- **34 total tests** covering search quality metrics and non-regression
- **11 French automotive queries** ported from Python specs (`specs/fix_search/`)
- **Metrics**: Precision, Recall, F1-Score, Response Quality Assessment

## 🚀 Quick Start

```bash
# Run all search quality tests
cargo test --package edgequake-query --test search_quality_tests

# Run with output (verbose)
cargo test --package edgequake-query --test search_quality_tests -- --nocapture

# Run specific query test
cargo test --package edgequake-query --test search_quality_tests test_q1_stla_byd_partnership
```

## 📁 Test Structure

```
edgequake/crates/edgequake-query/tests/
├── README.md                    # This file
├── metrics.rs                   # Precision/Recall/F1 calculation module
├── test_queries.rs              # 11 French automotive test queries
├── search_quality_tests.rs      # Main test suite (34 tests)
└── e2e_sota_engine.rs           # Existing SOTA engine E2E tests
```

## 📊 Metrics Module (`metrics.rs`)

Provides search quality measurement:

- `precision(retrieved, relevant)` - Precision calculation
- `recall(retrieved, relevant)` - Recall calculation
- `f1_score(precision, recall)` - F1 score (harmonic mean)
- `ResponseQuality::assess(response)` - Quality level assessment
- `TestSuiteMetrics` - Aggregate metrics across test runs

### Quality Levels

| Level     | Response Length | Description                    |
| --------- | --------------- | ------------------------------ |
| Excellent | > 1500 chars    | Comprehensive response         |
| Good      | 1000-1500 chars | Solid, detailed response       |
| Partial   | 500-1000 chars  | Acceptable but could be better |
| TooShort  | 200-500 chars   | Needs improvement              |
| NoInfo    | < 200 chars     | Inadequate response            |

## 🔍 Test Queries (`test_queries.rs`)

11 French automotive industry queries covering:

| ID                   | Theme                | Mode   | Description                       |
| -------------------- | -------------------- | ------ | --------------------------------- |
| Q1_STLA_BYD          | competitive_analysis | hybrid | Stellantis-BYD partnership        |
| Q2_VW_SCOUT          | competitive_analysis | hybrid | VW's Scout American SUV           |
| Q3_TOYOTA_HYBRID     | technology           | hybrid | Toyota hybrid technology strategy |
| Q4_PORSCHE_MACAN     | technology           | hybrid | Electric Porsche Macan            |
| Q5_TESLA_WARRANTY    | warranty             | hybrid | Tesla 8-year warranty concerns    |
| Q6_CUPRA_RAVAL       | pricing              | hybrid | Cupra Raval affordable EV         |
| Q7_NISSAN_RENAULT    | competitive_analysis | hybrid | Nissan-Renault alliance           |
| Q8_KIA_EV9           | technology           | hybrid | Kia EV9 SUV                       |
| Q9_MERCEDES_EQS      | technology           | hybrid | Mercedes EQS technology           |
| Q10_BMW_BATTERIES    | powertrain           | hybrid | BMW solid-state batteries         |
| Q11_DRIVING_DYNAMICS | powertrain           | hybrid | Driving dynamics comparison       |

## 🧪 Test Categories

### Individual Query Tests

Each of the 11 queries has a dedicated test that:

1. Creates a mock engine with automotive domain data
2. Runs the query through SOTAQueryEngine
3. Validates response quality (not NoInfo)

### Suite Tests

- `test_full_suite_quality_metrics` - Runs all 11 queries and reports aggregate metrics
- `test_suite_minimum_quality_bar` - Enforces minimum quality standards

### Regression Prevention Tests

- `test_hybrid_mode_returns_results` - Ensures hybrid mode works
- `test_french_queries_detected` - Validates French language detection
- `test_response_contains_domain_content` - Verifies domain relevance
- `test_empty_query_handled` - Edge case handling
- `test_special_characters_in_query` - Unicode handling

## 🔧 Technical Notes

### SOTAQueryEngine Pattern

Tests use `SOTAQueryEngine::with_mock_keywords()` which bypasses LLM-based keyword extraction for deterministic testing:

```rust
let engine = SOTAQueryEngine::with_mock_keywords(
    llm_provider.clone(),
    embed_provider,
    vector_storage,
    graph_storage,
    SOTAQueryConfig::default(),
);
```

### MockProvider Pattern

The same `MockProvider` implements both `LLMProvider` and `EmbeddingProvider`:

```rust
let provider = Arc::new(MockProvider::new());
// For embedding: provider.clone()
// For LLM: provider
```

### Storage Setup

```rust
let vector_storage = Arc::new(MemoryVectorStorage::new("test", 1536));
vector_storage.initialize().await?;

let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
```

## 📈 Running the Full Suite

```bash
# Quick run (results only)
cd edgequake && cargo test --package edgequake-query --test search_quality_tests 2>&1 | grep -E "test result|passed"

# Full verbose run
cd edgequake && cargo test --package edgequake-query --test search_quality_tests -- --nocapture

# Count passed tests
cd edgequake && cargo test --package edgequake-query --test search_quality_tests 2>&1 | grep -c " ok$"
```

## 🔗 Related Specs

These tests are based on specifications in:

- `specs/fix_search/` - Original Python query definitions
- `specs/fix_search/test_queries.py` - Python test queries
- `sessions/fix_search/` - Session logs from query improvement work

## ⚠️ Notes on `query.rs`

Investigation confirmed that `edgequake-core/src/query.rs` is **NOT obsolete**:

1. **Used by orchestrator.rs** (lines 266, 404)
2. **Different from edgequake-query::QueryEngine** - two separate implementations
3. **API uses edgequake-query crate** - `SOTAQueryEngine` and `QueryEngine` from edgequake-query
4. **Cannot remove without refactoring** - orchestrator.rs depends on it

### Two QueryEngine Implementations

| Crate           | File               | Used By            |
| --------------- | ------------------ | ------------------ |
| edgequake-core  | src/query.rs       | orchestrator.rs    |
| edgequake-query | src/engine.rs      | API, examples      |
| edgequake-query | src/sota_engine.rs | API (SOTA queries) |
