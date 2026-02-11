# Iteration 11 — Orient

## Analysis

### Architecture Decision: Pydantic v2 + httpx

- **Correct choice**: Pydantic v2 provides fast validation, `model_validate()` for dict→model, `model_dump()` for model→dict.
- httpx gives native async support, streaming, and connection pooling.
- `cached_property` on client avoids resource re-creation per access.

### Key Technical Patterns Validated

1. **Resource creation methods take Pydantic model objects** — e.g., `entities.create(EntityCreate(...))` not keyword args. This matches the TypeScript SDK pattern and ensures type safety.
2. **Sub-resources via properties** — `client.graph.entities` and `client.graph.relationships` are accessible, but also promoted to top-level: `client.entities`, `client.relationships`.
3. **PATCH operations** use `self._transport.request("PATCH", ...)` directly since base class lacks `_patch` helper.
4. **Async variants** required for every sync resource — discovered `ChunksResource` and `ProvenanceResource` lacked async counterparts.

### Risk Assessment

- **Field name mismatches**: Actual Pydantic models have different field names than initial assumptions (e.g., `id` vs `document_id`, `source`/`target` vs `source_id`/`target_id`, `track_id` required on TaskInfo). All resolved by reading actual type definitions.
- **187 tests**: Comprehensive coverage of types, resources, transport, streaming, pagination.

### Comparison with TypeScript SDK

| Aspect     | TypeScript                   | Python                  |
| ---------- | ---------------------------- | ----------------------- |
| Tests      | 415+                         | 187                     |
| Coverage   | 98.12%                       | ~90%+ (estimated)       |
| Resources  | 22 namespaces                | 22 namespaces           |
| Auth       | JWT + API key + multi-tenant | Same                    |
| Streaming  | SSE via EventSource          | SSE via httpx streaming |
| Pagination | Cursor-based                 | Cursor-based            |
