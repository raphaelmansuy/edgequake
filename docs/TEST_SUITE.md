# EdgeQuake Test Suite Documentation

## Overview

EdgeQuake has a comprehensive, multi-layer test suite designed as an **inviolable security layer** following first-principles reliability theory.

## Test Pyramid

```
                    ┌───────────────┐
                    │  Playwright   │ 643 tests
                    │    E2E        │ User workflows
                   ─┴───────────────┴─
                  ┌─────────────────────┐
                  │   API E2E Tests     │ 415 tests
                  │  (Rust, mocked)     │ API contracts
                 ─┴─────────────────────┴─
                ┌───────────────────────────┐
                │    Integration Tests      │ 57 tests
                │  (Invariants + existing)  │ Component interaction
               ─┴───────────────────────────┴─
              ┌─────────────────────────────────┐
              │         Unit Tests              │ 2,760+ tests
              │   (Core + Invariants + Edge)    │ Business logic
             ─┴─────────────────────────────────┴─
```

## Quick Commands

```bash
# Run all tests
cargo test --workspace

# Run only invariant tests
cargo test -p edgequake-core --test inviolable_invariants
cargo test -p edgequake-core --test edge_case_invariants
cargo test -p edgequake-api --test integration_invariants

# Run with timing
time cargo test --workspace

# Run single-threaded (isolation check)
cargo test --workspace -- --test-threads=1

# Run Playwright E2E
cd edgequake_webui && npx playwright test
```

## Invariant Tests

### Location: `edgequake/crates/edgequake-core/tests/inviolable_invariants.rs`

12 unit-level invariant tests covering critical system assumptions:

| ID      | Test Name                                     | Purpose                  |
| ------- | --------------------------------------------- | ------------------------ |
| INV-001 | `inv_001_chunk_size_within_embedding_limits`  | Chunks ≤ 8192 tokens     |
| INV-002 | `inv_002_workspace_isolation`                 | No cross-tenant data     |
| INV-003 | `inv_003_provider_resolution_respects_config` | Config → provider        |
| INV-004 | `inv_004_graph_edges_have_valid_nodes`        | No dangling edges        |
| INV-005 | `inv_005_api_requires_auth`                   | Auth except /health      |
| INV-006 | `inv_006_llm_errors_never_panic`              | Graceful error handling  |
| INV-007 | `inv_007_streaming_has_timeout`               | Max 60s streaming        |
| INV-008 | `inv_008_embeddings_are_deterministic`        | Same input → same output |
| INV-009 | `inv_009_pipeline_is_resumable`               | Checkpoint support       |
| INV-010 | `inv_010_query_timeout_is_configurable`       | Configurable timeouts    |

### Location: `edgequake/crates/edgequake-core/tests/edge_case_invariants.rs`

32 edge case tests covering boundary conditions:

- Empty inputs, max values, zero values
- Special characters, unicode
- Similar prefixes, duplicates
- Invalid/malformed inputs

### Location: `edgequake/crates/edgequake-api/tests/integration_invariants.rs`

7 integration-level tests:

- Workspace isolation at API level
- Provider resolution from config
- Auth middleware validation
- Error response sanitization
- Idempotency handling
- Timeout enforcement

## Speed Targets

| Category       | Target | Actual | Status |
| -------------- | ------ | ------ | ------ |
| Unit Tests     | <30s   | ~8s    | ✅     |
| Integration    | <2min  | <1s    | ✅     |
| API E2E        | <2min  | <5s    | ✅     |
| Playwright E2E | <5min  | TBD    | 🔍     |

## First Principles

1. **Falsifiability**: Every test can definitively fail
2. **Speed**: Tests complete in <30 seconds
3. **Isolation**: No shared state between tests
4. **Determinism**: Same results every run
5. **Coverage**: All business logic tested

## Adding New Invariant Tests

1. Identify the critical assumption
2. Assign an ID (INV-XXX)
3. Add unit test to `inviolable_invariants.rs`
4. Add edge cases to `edge_case_invariants.rs`
5. Add integration test if API-level
6. Update this documentation

## CI Integration

### Recommended workflow:

```yaml
name: Test Suite
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Tests
        run: cargo test --workspace

      - name: Verify Test Count
        run: |
          count=$(cargo test --workspace 2>&1 | grep -E "^test result:" | awk '{sum+=$4} END {print sum}')
          if [ $count -lt 2700 ]; then
            echo "Test count dropped below 2700!"
            exit 1
          fi
```

## Test Counts by Crate

| Crate                  | Tests | Type                     |
| ---------------------- | ----- | ------------------------ |
| edgequake-core         | 121+  | Unit + Invariants + Edge |
| edgequake-llm          | 199   | Unit                     |
| edgequake-storage      | 27    | Unit                     |
| edgequake-api          | 428+  | Unit + Integration       |
| edgequake-pipeline     | 94    | Unit                     |
| edgequake-query        | 82    | Unit                     |
| edgequake-pdf          | 398   | Unit                     |
| edgequake-rate-limiter | 50+   | Unit + Integration       |
| Other crates           | ~200  | Various                  |

## References

- Mission Spec: `specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md`
- OODA Loop 286-292: `specs/032-ollama-lmstudio-provider/ooda_loop/`
- AGENTS.md: Project guidelines including test requirements
