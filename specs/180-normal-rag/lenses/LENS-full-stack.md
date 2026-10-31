# Lens — Full stack

Parent: [README](../README.md) · Spec: [05-profile-law](../05-profile-law.md), [07-ingest-pipeline](../07-ingest-pipeline.md), [10-api-contract](../10-api-contract.md)

## View

The worker is one pipeline with gates. A second pipeline would drift the moment PDF admission or delete changes. `ProfileCapabilities` is constructed once per job and passed down. Handlers return the coded errors in [10-api-contract](../10-api-contract.md). They do not string-match the trigger message except to map `rag_profile_immutable` when the trigger fires first.

## Risks

| Risk | What we do |
|------|------------|
| `IngestProfile::from_env` still applied inside `PipelineConfig::from_env` | W1 stops the hot path from reading that env for a workspace that has a column. Unit test `T-180-env-ignored`. |
| Snapshot replay rebuilds a graph | W2 ignores snapshots when `extracts_graph` is false. |
| `partial_failure` left in one of two status writers | Both [persist.rs](../../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) and [processing.rs](../../../edgequake/crates/edgequake-pipeline/src/pipeline/processing.rs) are in the touch list. |
| In-memory service forgets the column | `T-180-memory-immutable`. |

## Acceptance

- Create, update, upload, reprocess, delete, and query honor the same struct.
- Graph tests listed in [16-e2e-test-matrix](../16-e2e-test-matrix.md) stay green.
- OpenAPI snapshot matches the DTOs.

## Laws owned

LAW-180-2, LAW-180-13, LAW-180-14, LAW-180-15. WHY-D.
