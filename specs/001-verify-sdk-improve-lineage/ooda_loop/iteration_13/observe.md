# OODA-13 Observe: Kotlin SDK Lineage Tests

## Current State
- Kotlin SDK at `sdks/kotlin/` with 99 existing unit tests
- Uses Maven + JUnit 5 + FakeHttpClient with reflection injection
- Data classes in `src/main/kotlin/io/edgequake/sdk/models/`
- No lineage-specific test coverage

## Observations
- Entity, Document, GraphNode/Edge, TaskInfo models have lineage-related fields
- Field names differ from Java: `deleted` vs `deletedCount`, `id` vs `trackId`, `label` vs `edgeType`
- EntityService.merge takes `(source: String, target: String)` not a request object
- ProviderStatus uses map-based structure (`provider`, `embedding`, `storage`, `metadata`)
