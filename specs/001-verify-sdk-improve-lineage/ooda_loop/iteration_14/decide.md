# OODA-14 Decide: Go SDK Lineage Test Plan

## Decision
Add 15 lineage tests to `sdks/go/edgequake_coverage_test.go`

## Tests
1. Entity fields (SourceID, Metadata, CreatedAt, Degree)
2. CreateEntityParams with SourceID and Metadata
3. ProvenanceRecord fields (Confidence, ExtractionMethod)
4. LineageGraph structure (Nodes, Edges, RootID)
5. LineageNode/Edge JSON roundtrips
6. CreateEntity sends sourceID in body
7. Neighborhood request with depth
