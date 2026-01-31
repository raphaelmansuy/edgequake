# Iteration 03: Orient

## Gap Analysis

| Current State                            | Desired State                                           | Gap                                 | Priority |
| ---------------------------------------- | ------------------------------------------------------- | ----------------------------------- | -------- |
| `extract()` method has no callback       | `extract_with_progress()` calls callbacks per page      | Need new method on PdfBackend trait | HIGH     |
| Sequential loop doesn't report progress  | Each page triggers `on_page_start` + `on_page_complete` | Inject callbacks in loop            | HIGH     |
| Parallel mode runs silently              | Parallel mode also calls callbacks (out-of-order is OK) | Call callbacks in rayon iterator    | MEDIUM   |
| PdfExtractor has `extract_to_markdown()` | Need `extract_to_markdown_with_progress()`              | Add wrapper method                  | HIGH     |

## Risk Assessment

- **Risk 1**: Breaking existing API - Mitigation: Add new method with default impl, don't modify signature
- **Risk 2**: Callback blocks main thread - Mitigation: Callbacks should be fast; document this requirement
- **Risk 3**: Parallel callbacks cause data races - Mitigation: `ProgressCallback: Send + Sync` already required
- **Risk 4**: Out-of-order callbacks in parallel mode - Mitigation: Document this behavior; UI can sort by page_num

## First Principles Analysis

- **Core problem**: No visibility into page-level extraction progress
- **Fundamental constraint**: LopdfDocument is not Sync, so parallel mode reloads PDF per thread
- **Minimal solution**: Add optional callback param with default no-op implementation
- **Why this matters**: Users see "Processing..." for 30+ seconds on large PDFs without feedback

## Alternative Approaches

1. **Option A: Add callback to existing `extract()` method**
   - Pros: Single method to maintain
   - Cons: Breaking change; requires all callers to update; awkward API with Optional<Arc<dyn...>>

2. **Option B: Add new `extract_with_progress()` method with default impl** ✅ CHOSEN
   - Pros: Non-breaking; clear intent; default falls back to `extract()`
   - Cons: Two methods to maintain

3. **Option C: Builder pattern with `.with_progress(callback).extract()`**
   - Pros: Fluent API
   - Cons: More complex implementation; state management issues

## Architecture Decision

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CHOSEN: Option B - New Method                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PdfBackend trait:                                                          │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ async fn extract(&self, bytes) -> Result<Document>  // existing     │  │
│  │                                                                      │  │
│  │ async fn extract_with_progress(                     // NEW          │  │
│  │     &self,                                                           │  │
│  │     bytes: &[u8],                                                   │  │
│  │     callback: Arc<dyn ProgressCallback>,                            │  │
│  │ ) -> Result<Document> {                                             │  │
│  │     self.extract(bytes).await  // default impl                      │  │
│  │ }                                                                    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ExtractionEngine: Override with real callback calls in page loop          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Complexity

- PdfBackend trait: +12 lines (new method with default impl)
- ExtractionEngine: +50 lines (implement with callback calls in loops)
- PdfExtractor: +30 lines (wrapper method)
- Tests: +50 lines (verify callbacks are called)

Total: ~150 lines of code
