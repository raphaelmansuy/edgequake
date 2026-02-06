# OODA-14 Observe: Re-indexing E2E Tests

## Findings

1. **Duplicate Detection**: Upload handler (documents.rs:536) uses SHA-256 content hash scoped by workspace to detect duplicates. Returns 200 OK with `status: "duplicate"` and `duplicate_of` field.

2. **Reprocess Endpoint**: POST /api/v1/documents/reprocess (documents.rs:3372) requires valid UUID tenant_id from X-Tenant-ID header. Without it, defaults to "default" → `Uuid::parse_str("default")` fails → 422.

3. **Graph Storage**: Mock provider returns "Mock response" text, not valid JSON entities. Entity extraction produces 0 entities → 0 graph nodes in test mode. Graph structure is still valid (empty arrays).

4. **Test State**: `AppState::test_state()` uses MemoryGraphStorage, MockProvider, and Pipeline::default_pipeline(). No real entity extraction occurs.

## Key Code Paths

- Duplicate detection: `documents.rs:520-570`
- Reprocess handler: `documents.rs:3372-3540`
- Task UUID validation: `documents.rs:640` (Uuid::parse_str)
- TenantContext extraction: `middleware.rs:366-420`
