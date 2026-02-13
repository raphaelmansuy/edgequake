# OODA-14 Observe: Go SDK Lineage Tests

## Current State
- Go SDK at `sdks/go/` with 194 unit tests (56 base + 138 coverage)
- Uses `httptest.NewServer` for mocking
- Already has some lineage tests (lineage depth, provenance for entity)
- Types in `types.go`: Entity, ProvenanceRecord, LineageGraph, LineageNode, LineageEdge
