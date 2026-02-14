# OODA-19: Observe — Java SDK Missing LineageService

## Finding

- Java SDK had 19 services but no LineageService
- 7 lineage endpoints have no Java SDK methods: entityLineage, documentLineage, documentFullLineage, exportLineage, chunkDetail, chunkLineage, entityProvenance
- EdgeQuakeClient had no `lineage()` accessor

## Orient

- Created LineageService.java with 7 methods covering all lineage endpoints
- Added lineageService to EdgeQuakeClient constructor + accessor

## Decide

1. Create LineageService.java (7 methods, 7 endpoints)
2. Add LineageService to EdgeQuakeClient
3. Add `assertNotNull(client.lineage())` test
4. Run tests to verify

## Act

- Created `sdks/java/src/main/java/io/edgequake/sdk/resources/LineageService.java` (7 methods)
- Updated `sdks/java/src/main/java/io/edgequake/sdk/EdgeQuakeClient.java` (+lineageService field, constructor, accessor)
- Updated `sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java` (+lineage() assertion)
- Tests: 123 pass, 0 fail
- Commit: pending
