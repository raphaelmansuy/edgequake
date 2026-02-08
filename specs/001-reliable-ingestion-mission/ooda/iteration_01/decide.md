# Iteration 01 - Decide

**Date**: 2026-02-08  
**Mission**: Reliable Document Ingestion Pipeline

## Decision

**Proceed with Option A: Minimal Change**

## Prioritized Action Plan

### Priority 1: Fix gpt-4o-mini → gpt-5-nano (HIGH)

**Why:** Immediate blocker - OpenAI quota exceeded for gpt-4o-mini

| #   | File                                                 | Change                           | Impact         |
| --- | ---------------------------------------------------- | -------------------------------- | -------------- |
| 1   | `edgequake/docs/configuration.md`                    | Update model references          | Documentation  |
| 2   | `edgequake/models.toml`                              | Update comment                   | Config example |
| 3   | `edgequake-pipeline/src/lineage.rs`                  | Update default model in comments | Code docs      |
| 4   | `edgequake-pipeline/tests/cost_integration_tests.rs` | Update test model references     | Tests          |

### Priority 2: Verify LLM Provider Factory (MEDIUM)

**Why:** Ensure default model selection uses gpt-5-nano when OpenAI is selected

| #   | File                               | Action                        |
| --- | ---------------------------------- | ----------------------------- |
| 5   | `edgequake-llm/src/providers/*.rs` | Check default model constants |
| 6   | `edgequake-llm/src/factory.rs`     | Verify factory logic          |

### Priority 3: Add Memory Mode Warning (LOW)

**Why:** Prevent accidental production use of memory storage

| #   | File                    | Change                                      |
| --- | ----------------------- | ------------------------------------------- |
| 7   | `edgequake/src/main.rs` | Add explicit warning banner for memory mode |

### Priority 4: Test Complete Pipeline (VERIFICATION)

| #   | Test                             |
| --- | -------------------------------- |
| 8   | Upload PDF via UI                |
| 9   | Verify entity extraction         |
| 10  | Check Knowledge Graph Population |
| 11  | Run unit tests                   |

## Out of Scope (for this iteration)

- Removing in-memory providers entirely
- Cost calculation updates for gpt-5-nano (prices may not be public yet)
- Feature flags for storage modes

## Success Criteria

1. ✅ All `gpt-4o-mini` references updated to `gpt-5-nano`
2. ✅ Memory mode has visible warning at startup
3. ✅ Document upload + entity extraction works
4. ✅ All existing tests pass

## Rollback Plan

If issues arise:

1. Revert model name changes
2. Keep memory mode warning (low risk)
3. Document any API differences discovered
