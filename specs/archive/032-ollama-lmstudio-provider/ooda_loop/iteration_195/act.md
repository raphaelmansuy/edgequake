# OODA 195: Act - PostgreSQL Provider Switching Tests

**Date**: 2025-01-15
**Focus**: Implementation of PostgreSQL-specific E2E tests

## Test File Created

[e2e_postgres_provider_switching.rs](../../../../edgequake/crates/edgequake-api/tests/e2e_postgres_provider_switching.rs)

## Test Results

| Test Name                                       | Status  |
| ----------------------------------------------- | ------- |
| test_provider_config_persists_to_postgres       | ✅ PASS |
| test_provider_update_persists_to_postgres       | ✅ PASS |
| test_empty_metadata_uses_defaults               | ✅ PASS |
| test_provider_factory_respects_workspace_config | ✅ PASS |
| test_openai_provider_fails_without_api_key      | ✅ PASS |
| test_multiple_workspaces_provider_isolation     | ✅ PASS |
| test_provider_switch_ollama_to_openai           | ✅ PASS |
| test_embedding_dimension_update                 | ✅ PASS |

**Total: 8 tests, 8 passed, 0 failed**

## Key Verifications

1. **Persistence**: Provider config (llm_provider, embedding_provider, dimension) correctly stored in metadata JSONB
2. **Updates**: JSONB merge operator (`||`) correctly updates provider fields
3. **Defaults**: Empty metadata falls back to ollama/gemma3 defaults
4. **Isolation**: Multiple workspaces maintain separate provider configurations
5. **Provider Switch**: Can switch from ollama to openai and vice versa
6. **Dimension Tracking**: embedding_dimension correctly persists and updates

## Command to Run

```bash
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"
cargo test --package edgequake-api --test e2e_postgres_provider_switching --features postgres
```

## Next Steps

- OODA 196-200: Add provider lineage tracking for extractions
- Track which provider was actually used for each document extraction
- Display lineage in document metadata
