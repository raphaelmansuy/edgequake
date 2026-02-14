# OODA-31: Observe

## Focus: Rust SDK — Lineage + Settings Resources

### Observations

1. **Provenance URL bug**: `provenance.rs` `lineage()` pointed to `/api/v1/entities/{}/lineage` — route does NOT exist in backend. Correct route: `/api/v1/lineage/entities/{name}`.
2. **Missing dedicated resources**: No `lineage.rs` or `settings.rs` resource modules — lineage was partially in provenance, settings was missing entirely.
3. **Missing `get_raw()` on client**: Export lineage returns raw bytes (JSON/CSV), but client only had `get()` returning deserialized JSON.
4. **Existing test used wrong path regex**: `test_lineage_via_provenance_service` mocked `/api/v1/entities/.+/lineage` — never matched real backend.
5. **Test count**: 156 tests before changes.

### Data Points

- Backend routes verified: `/api/v1/lineage/entities/{name}`, `/api/v1/lineage/documents/{id}`, `/api/v1/documents/{id}/lineage`, `/api/v1/documents/{id}/lineage/export`, `/api/v1/settings/provider/status`, `/api/v1/settings/providers`
- Rust SDK had 21 resource accessors, missing lineage + settings
