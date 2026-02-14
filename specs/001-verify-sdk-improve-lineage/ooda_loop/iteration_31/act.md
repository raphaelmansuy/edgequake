# OODA-31: Act

## Changes Made

### Files Modified
- `sdks/rust/src/resources/provenance.rs` — Fixed URL from `/api/v1/entities/{}/lineage` to `/api/v1/lineage/entities/{}`
- `sdks/rust/src/client.rs` — Added `get_raw()` method + `lineage()` and `settings()` accessors (21→23 resources)
- `sdks/rust/src/resources/mod.rs` — Added `pub mod lineage;` and `pub mod settings;`
- `sdks/rust/tests/integration_tests.rs` — Fixed test path regex, added 10 new tests, imported `query_param`

### Files Created
- `sdks/rust/src/resources/lineage.rs` — 4 methods: entity_lineage, document_lineage, document_full_lineage, export_lineage
- `sdks/rust/src/resources/settings.rs` — 2 methods: provider_status, list_providers

### Test Results
- **Before**: 156 tests (85 integration + 70 unit + 1 doc)
- **After**: 166 tests (85 integration + 80 unit + 1 doc) — 10 new tests added
- **All passing**: `test result: ok. 166 passed; 0 failed`

### New Tests Added
1. `test_lineage_entity_lineage` — entity lineage with nodes+edges
2. `test_lineage_entity_lineage_url_encodes_name` — URL encoding for spaces
3. `test_lineage_document_lineage` — document lineage graph
4. `test_lineage_document_full_lineage` — full document lineage with metadata
5. `test_lineage_export_json` — JSON export returns raw bytes
6. `test_lineage_export_csv` — CSV export returns raw bytes
7. `test_lineage_empty_graph` — empty lineage graph edge case
8. `test_settings_provider_status` — provider status with all fields
9. `test_settings_list_providers` — list available providers
10. `test_settings_provider_status_no_provider` — no provider configured edge case
