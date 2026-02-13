# Observation - Iteration 18

## Focus: API Reference Documentation for Lineage Endpoints

## Files Examined

- `edgequake/crates/edgequake-api/src/handlers/lineage.rs` (855 lines) — All lineage handler implementations
- `edgequake/crates/edgequake-api/src/handlers/lineage_types.rs` (~595 lines) — DTO response types
- `edgequake/crates/edgequake-api/src/routes.rs` — Route registrations

## Current State

- 7 lineage-related API endpoints exist in the codebase
- All have utoipa annotations for OpenAPI generation
- No standalone API reference document existed for developers

## Endpoints Documented

1. `GET /documents/{id}/lineage` — Complete lineage tree (OODA-07)
2. `GET /documents/{id}/metadata` — Flat metadata (OODA-07)
3. `GET /chunks/{chunk_id}` — Chunk detail
4. `GET /chunks/{chunk_id}/lineage` — Chunk lineage (OODA-08)
5. `GET /entities/{entity_id}/provenance` — Entity provenance
6. `GET /lineage/entities/{entity_name}` — Entity lineage
7. `GET /lineage/documents/{document_id}` — Document graph lineage
