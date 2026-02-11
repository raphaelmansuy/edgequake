# Iteration 06 — Orient

## Analysis: SDK vs Reality Gap

### First Principles Assessment

The SDK was designed from API _documentation_ (OpenAPI spec, type definitions). E2E testing against the _actual_ backend revealed discrepancies between documented and actual behavior. This is expected — the SDK must ultimately conform to the running server, not static docs.

### Categorization of Issues

**Category 1: Transport Layer (1 fix)**

- Content-Type detection: The transport must handle both JSON and text responses. The fix is universal — affects all endpoints.

**Category 2: Response Shape Mismatches (3 fixes)**

- Documents list: `{documents:[]}` vs `{items:[]}`
- Entities list: paginated object vs raw array
- Relationships list: same as entities

**Category 3: API Contract Mismatches (2 fixes)**

- CreateEntityRequest field names differ from Rust struct
- Entity exists endpoint uses `entity_name` not `name`

**Category 4: Streaming Protocol (1 fix)**

- Query stream sends raw text in SSE, not JSON events
- `_streamSSE()` was designed for JSON-based SSE only

**Category 5: Auth Requirements (awareness, not fix)**

- Chat/conversation endpoints require tenant context (X-Tenant-ID, X-User-ID)
- SDK has tenantId config but no userId — needs addition in future iteration

### Risk Assessment

| Fix                      | Risk                                   | Mitigation                            |
| ------------------------ | -------------------------------------- | ------------------------------------- |
| Content-Type detection   | May affect binary downloads            | Only falls back to text when not JSON |
| List response extraction | Breaking if API ever returns raw array | Fallback: `raw.items ?? raw as T[]`   |
| Stream raw text handling | Different endpoints may mix formats    | Try JSON first, fall back to text     |
| Chat tenant context      | Tests skip gracefully                  | Add userId config in iteration 07     |

### Patterns Learned

1. **Always test against live backend** — unit tests with mocks can't catch API shape issues
2. **API responses are often paginated wrappers** — SDK list methods need to unwrap
3. **SSE streams aren't always JSON** — need format-flexible stream parsing
4. **Multi-tenant APIs need explicit identity headers** — discovery through 401 errors
