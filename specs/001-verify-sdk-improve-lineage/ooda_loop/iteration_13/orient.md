# OODA-13 Orient: Kotlin SDK Lineage Gap Analysis

## Gap Analysis
- 0 tests for entity source_id, metadata, timestamps
- 0 tests for GraphNode/Edge provenance properties
- 0 tests for Document lineage fields (title, chunkCount)
- 0 tests for BulkDeleteResponse.deleted
- 0 tests for TaskInfo.id field

## Field Name Mapping (Kotlin vs Java)
| Kotlin | Java | Notes |
|--------|------|-------|
| `deleted` | `deletedCount` | BulkDeleteResponse |
| `id` | `trackId` | TaskInfo |
| `label` | `edgeType` | GraphEdge |
| `provider` map | `currentProvider` | ProviderStatus |

## Priority
- Add 23 lineage/metadata tests matching actual Kotlin data class fields
