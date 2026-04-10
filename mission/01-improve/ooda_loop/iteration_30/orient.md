# OODA-30 — Orient

## Analysis

### First Principles

1. **Builder methods are pure functions** — every single one can be tested without mocks
2. **`with_llm_full_id` has parsing logic** — must test: with slash, without slash, multiple slashes, empty string
3. **StrategyConfig defaults define system behavior** — untested defaults are invisible contracts
4. **QueryEngineConfig defaults are partially tested** — 6 of 9 fields unchecked = silent regression risk

### Risk Assessment

| Risk                                          | Impact                        | Mitigation                    |
| --------------------------------------------- | ----------------------------- | ----------------------------- |
| `with_llm_full_id` misparses multi-slash IDs  | Medium — wrong model selected | Test "provider/model/version" |
| Strategy weights sum != 1.0 not validated     | Low — weights are advisory    | Document in WHY comment       |
| QueryEngineConfig truncation default untested | Medium — could affect context | Assert truncation default     |

### Approach

- Add WHY comment to StrategyConfig explaining weight semantics
- Add WHY comment to QueryEngineConfig explaining alignment with SOTAQueryConfig
- Add comprehensive tests for all untested builder methods
- Focus on `with_llm_full_id` edge cases (most complex parsing logic)
