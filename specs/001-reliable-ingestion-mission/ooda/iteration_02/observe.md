# OODA Iteration 02 - Observe

## Mission Re-Read Checkpoint

✅ Mission file read: `./specs/001-reliable-ingestion-mission.md`

## Observation Summary

### 1. Service Status

- **Backend**: ✅ Healthy (http://localhost:8080/health)
  - Storage mode: postgresql
  - LLM provider: ollama
  - Schema version: 23, migrations: 22
- **Frontend**: ✅ Running (http://localhost:3000)

### 2. Remaining gpt-4o-mini References in Code

Despite iteration 01 claiming migration was complete, **critical references remain**:

| File                                 | Line    | Context                                            |
| ------------------------------------ | ------- | -------------------------------------------------- |
| `edgequake-pipeline/src/cache.rs`    | 297     | `// Estimate savings based on gpt-4o-mini pricing` |
| `edgequake-pipeline/src/cache.rs`    | 389     | `CacheEntry::new(..., "gpt-4o-mini")` (test)       |
| `edgequake-pipeline/src/progress.rs` | 610-613 | `new_gpt4o_mini()` constructor                     |
| `edgequake-pipeline/src/progress.rs` | 659-660 | HashMap entry `"gpt-4o-mini"`                      |
| `edgequake-pipeline/src/progress.rs` | 774     | Test using gpt-4o-mini pricing                     |
| `edgequake-pipeline/src/progress.rs` | 798     | Test assertion for gpt-4o-mini                     |
| `edgequake/docs/configuration.md`    | 243     | `model = "gpt-4o"` example                         |
| `edgequake/models.toml`              | 53-54   | GPT-4o model definition                            |

### 3. Makefile Analysis

**Current State:**

- `DATABASE_URL` has a default value at line 332:
  ```makefile
  DATABASE_URL := postgresql://edgequake:edgequake_secret@localhost:5432/edgequake
  ```
- `make dev` and `make dev-bg` always pass this default
- `make backend-memory` does NOT set DATABASE_URL (⚠️ unsafe for production)

**Missing:**

- [ ] No explicit validation that fails if DATABASE_URL is unset during dev/production
- [ ] No test to ensure `make dev` never runs in memory mode

### 4. In-Memory Provider Status

In-memory providers exist in:

- `edgequake-storage/src/adapters/memory/` - Used when DATABASE_URL not set
- `main.rs:254` logs: `"💾 No DATABASE_URL set - using in-memory storage"`

**Decision from Iteration 01:** Keep in-memory providers for testing. However:

- **Missing:** Documentation warning that memory mode is NOT for production
- **Missing:** Test that validates dev mode requires DATABASE_URL

### 5. Success Criteria Checklist

| Criterion                               | Status | Notes                                |
| --------------------------------------- | ------ | ------------------------------------ |
| Document upload via UI works            | ✅     | Tested in iteration 01               |
| Document processing completes           | ✅     | 3 documents processed                |
| KG populated with entities              | ✅     | 200 entities, 11 connections         |
| No in-memory providers in prod path     | ⚠️     | Exist but not used with DATABASE_URL |
| gpt-5-nano is default OpenAI model      | ⚠️     | gpt-4o-mini refs remain in code      |
| All tests pass                          | ❓     | Unable to verify (tests timing out)  |
| No dead code/duplicates                 | ❓     | Not fully audited                    |
| SRP/DRY followed                        | ✅     | Code is modular                      |
| No hardcoded models                     | ❌     | gpt-4o-mini in tests/examples        |
| Pipeline recovers from errors           | ⚠️     | To verify                            |
| Edge cases handled                      | ❓     | Not fully tested                     |
| gpt-5-nano works for ingestion          | ⚠️     | Ollama used currently                |
| Memory mode documented as test-only     | ❌     | Missing documentation                |
| Makefile dev fails without DATABASE_URL | ❌     | No validation exists                 |

### 6. Code Structure ASCII Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         EdgeQuake Ingestion Pipeline                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   User Upload                                                                │
│       │                                                                      │
│       ▼                                                                      │
│   ┌───────────┐     ┌─────────────┐     ┌────────────────┐                  │
│   │  REST API │────▶│   TaskQueue │────▶│  PipelineWorker │                  │
│   │  (Axum)   │     │  (mpsc)     │     │  Pool (N=4)     │                  │
│   └───────────┘     └─────────────┘     └────────────────┘                  │
│                                               │                              │
│                        ┌──────────────────────┤                              │
│                        ▼                      ▼                              │
│               ┌─────────────┐        ┌────────────────┐                     │
│               │  PDF Extract │        │  Entity Extract │                    │
│               │  (pdfium)    │        │  (LLM: Ollama)  │                    │
│               └─────────────┘        └────────────────┘                     │
│                        │                      │                              │
│                        └────────┬─────────────┘                              │
│                                 ▼                                            │
│                        ┌────────────────┐                                   │
│                        │   PostgreSQL   │                                   │
│                        │   + pgvector   │                                   │
│                        │   + Apache AGE │                                   │
│                        └────────────────┘                                   │
│                                                                              │
│   Storage Mode Selection (main.rs):                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ if DATABASE_URL set:                                                │   │
│   │     → PostgreSQL (PRODUCTION)                                       │   │
│   │ else:                                                               │   │
│   │     → In-Memory (TEST ONLY ⚠️)                                      │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   LLM Provider Selection:                                                   │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ EDGEQUAKE_DEFAULT_LLM_PROVIDER env var:                            │   │
│   │     "ollama" → Ollama (local)                                      │   │
│   │     "openai" → OpenAI (gpt-5-nano)                                 │   │
│   │     unset   → fallback based on OPENAI_API_KEY presence            │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Key Gaps Identified

1. **gpt-4o-mini residue**: Tests and pricing code still reference deprecated model
2. **No DATABASE_URL validation**: Makefile has default but no guard against accidental memory mode
3. **Missing documentation**: Memory mode not explicitly documented as test-only
4. **Test verification**: Need to confirm all tests pass after changes
5. **No Makefile safety test**: Need test that verifies dev mode enforces DATABASE_URL

## Next Steps (Orient Phase)

1. Remove/replace remaining gpt-4o-mini references
2. Add DATABASE_URL validation to Makefile
3. Document memory mode as test-only
4. Create integration test for Makefile safety
5. Run full test suite to verify changes
