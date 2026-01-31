# OODA-08: Orient

## Gap Analysis

| Current State                            | Desired State                                  | Gap           | Priority |
| ---------------------------------------- | ---------------------------------------------- | ------------- | -------- |
| `extract_to_markdown()` without progress | `extract_to_markdown_with_progress(callback)`  | Need adapter  | HIGH     |
| No bridge between crates                 | Adapter in edgequake-api                       | Create struct | HIGH     |
| `on_progress(current, total, item_name)` | `emit_pdf_page_progress(pdf_id, task_id, ...)` | Map fields    | HIGH     |

## Risk Assessment

- **Risk 1**: Trait method signatures don't exactly match - Mitigation: Adapt `on_progress` to track page count
- **Risk 2**: `item_name` is String, need to parse page number - Mitigation: Use naming convention "page_N"
- **Risk 3**: Circular dependency api→pdf→tasks - Mitigation: Adapter in api, uses Arc closures

## First Principles Analysis

- **Core problem**: Need to convert `ProgressCallback` calls to `PipelineEvent::PdfPageProgress`
- **Fundamental constraint**: edgequake-pdf can't depend on edgequake-api
- **Minimal solution**: Struct in api that holds `PipelineState` + ids, implements trait
- **Why this matters**: Real-time page progress in WebSocket

## Alternative Approaches

1. **Option A: Struct with fields**

   ```rust
   pub struct BroadcastingProgressCallback {
       pipeline_state: PipelineState,
       pdf_id: String,
       task_id: String,
   }
   ```

   - Pros: Testable, explicit, reusable
   - Cons: Need to add edgequake-pdf as dependency (already is)

2. **Option B: Closure-based in processor.rs**
   - Pros: No new struct
   - Cons: Less testable, harder to maintain

**Chosen: Option A** - Clean struct-based adapter
