# OODA 192: Observe - PostgreSQL Provider Switching Analysis

**Date**: 2025-01-15
**Focus**: Analyzing PostgreSQL-specific provider switching requirements

## Current State

### Existing Tests (Memory Backend)

- [e2e_workspace_provider_ingestion.rs](../../../../edgequake/crates/edgequake-api/tests/e2e_workspace_provider_ingestion.rs): 11 tests passing
- [e2e_workspace_provider_rebuild.rs](../../../../edgequake/crates/edgequake-api/tests/e2e_workspace_provider_rebuild.rs): 6 tests passing

### PostgreSQL Testing Pattern

From [e2e_postgres_workspace.rs](../../../../edgequake/crates/edgequake-api/tests/e2e_postgres_workspace.rs):

- Uses `#[cfg(feature = "postgres")]` conditional compilation
- Requires `DATABASE_URL` or `POSTGRES_PASSWORD` environment variable
- Uses `require_postgres!()` macro to skip tests when DB unavailable
- Clean up test data after each test

## Key Observations

1. **Processor.rs Fix Applied**:

   - Lines 168-240 now use `create_safe_llm_provider` and `create_safe_embedding_provider`
   - Explicit ERROR-level logging for provider creation failures
   - Fallback to default pipeline with clear warning

2. **Provider Configuration Persistence**:

   - Workspace stores `llm_provider`, `llm_model`, `embedding_provider`, `embedding_model`, `embedding_dimension`
   - These values persist in PostgreSQL `workspaces` table
   - Need to verify they're correctly used after server restart

3. **PostgreSQL-Specific Concerns**:
   - Provider config must persist across database connections
   - Workspace updates must atomically update provider fields
   - Need to test provider switching with real database transactions

## Questions to Answer

1. Does changing workspace provider in DB immediately affect document processing?
2. Is there any caching that could cause stale provider config?
3. Do rebuild operations correctly read updated provider config from DB?

## Next Step

OODA 193: Orient - Design PostgreSQL-specific test scenarios for provider switching
