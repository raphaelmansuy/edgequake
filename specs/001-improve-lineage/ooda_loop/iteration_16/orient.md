# Analysis - Iteration 16

## Gap Identified

Python SDK lacked lineage retrieval methods, making it impossible for Python users to:
- Retrieve full document lineage (document + chunks + entities)
- Retrieve document metadata in a single call
- Retrieve chunk lineage with parent document references

Rust SDK (OODA-14) and TypeScript SDK (OODA-15) already had these methods.

## Solution Applied

### Approach: Mirror Rust/TS SDK pattern in Python

- **Pros**: Consistent API surface across all 3 SDKs, follows existing Pydantic model pattern
- **Cons**: None significant
- **Risk**: Low — additive changes only, no existing signatures modified

## Design Decisions

1. **Pydantic models** for `DocumentFullLineage` and `ChunkLineageInfo` — matches existing SDK pattern using `BaseModel` with `ConfigDict(extra="allow")` for forward compatibility
2. **Both sync + async** methods added to `DocumentsResource`/`AsyncDocumentsResource` and `ChunksResource`/`AsyncChunksResource`
3. **Docstrings** include HTTP method/path and `@implements` annotations for traceability

## Recommendation

Ship as-is — all 3 SDKs now have identical lineage capabilities. This completes deliverable #5 (F7).
