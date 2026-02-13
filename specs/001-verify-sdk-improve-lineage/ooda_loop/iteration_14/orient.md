# OODA-14 Orient: Go SDK Lineage Gap Analysis

## Existing Coverage
- TestLineageDepth, TestLineageNoDepth already exist
- TestProvenanceForEntity exists in base tests
- Entity struct has SourceID, Metadata fields

## Gaps
- No unit test for Entity.SourceID / Entity.Metadata fields
- No test for CreateEntityParams with SourceID
- No JSON roundtrip tests for ProvenanceRecord
- No tests for LineageGraph node/edge structure
- No tests for CreateEntity sending source_id in request body
