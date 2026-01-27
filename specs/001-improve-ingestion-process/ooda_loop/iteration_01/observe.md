# Iteration 01: Observe

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Territory Mapping

### 1. Frontend Architecture (edgequake_webui/src/components/documents/)

| File                             | Purpose                     | Lines | Key Functions                                 |
| -------------------------------- | --------------------------- | ----- | --------------------------------------------- |
| document-manager.tsx             | Main document management UI | 1094  | Upload, list, delete, reprocess               |
| reprocess-failed-button.tsx      | Batch retry failed docs     | 194   | `reprocessFailedDocuments()`                  |
| reset-document-status-button.tsx | Individual doc actions      | ~150  | `reprocessDocument()`, `retryTask()`          |
| pipeline-status-dialog.tsx       | Progress monitoring         | 326   | Shows stages, messages, progress              |
| status-badge.tsx                 | Document status display     | 42    | pending/processing/completed/failed/cancelled |
| batch-progress-card.tsx          | Batch upload progress       | -     | Multi-file upload tracking                    |
| ingestion-progress-panel.tsx     | Detailed ingestion stages   | -     | Step-by-step progress                         |

### 2. API Client (edgequake_webui/src/lib/api/edgequake.ts)

```
Lines 520-600: Document Operations
├── reprocessDocument(trackId)      → POST /documents/reprocess {track_id, max_documents: 1}
├── reprocessFailedDocuments()      → POST /documents/reprocess {}
├── scanDocuments(path?)            → POST /documents/scan
├── deleteDocument(id)              → DELETE /documents/{id}
└── deleteAllDocuments()            → DELETE /documents
```

### 3. Backend Architecture (edgequake/crates/edgequake-api/src/)

```
handlers/
├── documents.rs          3767 lines - Core document handlers
│   ├── upload_document()
│   ├── reprocess_failed()      Line 3145 - Batch retry handler
│   ├── scan_directory()        Line 2880
│   ├── cleanup_document_graph_data()   Line 276 - Graph cleanup before reprocess
│   └── get_workspace_vector_storage_strict()  Line 58 - Vector isolation
├── documents_types.rs    DTOs and request/response types
├── workspaces.rs         Workspace operations + rebuild endpoints
│   ├── reprocess_all_documents()   Line 2023 - Full workspace reprocess
│   └── rebuild_knowledge_graph()   Clears KG + embeddings
└── pipeline.rs           Pipeline status/control
```

### 4. Current Document Processing Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DOCUMENT INGESTION FLOW                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  [Upload File]  →  [Validate]  →  [Store Content]  →  [Queue Task]       │
│       │               │               │                  │               │
│       ▼               ▼               ▼                  ▼               │
│   multipart       size/type      KV: {id}-content   edgequake_tasks     │
│   form-data       extension      KV: {id}-metadata   TaskQueue          │
│                                                                          │
│  ─────────────────── ASYNC TASK PROCESSOR ──────────────────────────     │
│       │                                                                  │
│       ▼                                                                  │
│  [Chunk Document]  →  [Extract Entities]  →  [Generate Embeddings]       │
│       │                     │                       │                    │
│       ▼                     ▼                       ▼                    │
│   1200 tokens         LLM Provider           Embedding Provider          │
│   100 overlap         (OpenAI/Ollama)        (OpenAI/Ollama)            │
│                                                                          │
│       ▼                     ▼                       ▼                    │
│  [Store Chunks]  →  [Upsert Graph]  →  [Index Vectors]                  │
│       │                 │                    │                           │
│       ▼                 ▼                    ▼                           │
│   KV Storage       Graph Storage        Vector Storage                   │
│   {id}-chunks      nodes + edges        workspace-specific               │
│                                                                          │
│  ───────────────────── STATUS TRACKING ─────────────────────────────     │
│                                                                          │
│   pending → processing → completed/failed                                │
│      │          │            │                                          │
│      ▼          ▼            ▼                                          │
│   KV: {id}-metadata.status                                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5. Identified Issues

#### 5.1 Critical Bug (FIXED)

- **Location**: document-manager.tsx:677, 795
- **Issue**: `Loader2` icon used but not imported from lucide-react
- **Status**: ✅ Fixed in this session

#### 5.2 Reprocess Flow Gaps

1. **No progress visibility during reprocess** - User doesn't know what's happening
2. **No step-by-step feedback** - Chunking, extraction, embedding stages not shown
3. **Error messages lack context** - Generic "failed" without reason

#### 5.3 Rebuild Operations

- `rebuild_knowledge_graph()` in workspaces.rs clears KG but doesn't show progress
- `reprocess_all_documents()` queues all docs but no per-doc status
- No "rebuild embeddings only" functionality

#### 5.4 UX/UI Issues

1. Status badge shows only 5 states - no sub-states for processing
2. Pipeline dialog shows aggregate counts but not per-document details
3. Error messages stored in metadata but not displayed to user
4. No ETA calculation for processing time

### 6. Document Status States

Current states in status-badge.tsx:

```typescript
const statusConfig = {
  pending: { icon: Clock, color: "bg-yellow-500", label: "Pending" },
  processing: { icon: Loader2, color: "bg-blue-500", label: "Processing" },
  completed: { icon: CheckCircle, color: "bg-green-500", label: "Completed" },
  indexed: { icon: CheckCircle, color: "bg-green-500", label: "Indexed" },
  failed: { icon: XCircle, color: "bg-red-500", label: "Failed" },
  cancelled: { icon: StopCircle, color: "bg-orange-500", label: "Cancelled" },
};
```

**Missing states for better UX:**

- `chunking` - Splitting document into chunks
- `extracting` - Running LLM entity extraction
- `embedding` - Generating embeddings
- `indexing` - Storing in graph/vector DB

### 7. Workspace Isolation Check

From documents.rs line 58-180, workspace isolation is handled by:

```rust
async fn get_workspace_vector_storage_strict(
    state: &AppState,
    workspace_id: &str,
) -> Result<Arc<dyn VectorStorage>, ApiError>
```

Key behaviors:

- Production mode (PostgreSQL): STRICT - fails if workspace not found
- Memory mode (tests): FALLBACK allowed to default storage
- Workspace embedding dimension is preserved per-workspace

### 8. E2E Test Coverage Status

```
edgequake_webui/e2e/
├── documents/          Document-related tests (to check)
└── playwright.config.ts  Configuration
```

Need to verify existing test coverage and add Ollama-based tests.

---

## Key Files to Modify

| Priority | File                        | Changes Needed               |
| -------- | --------------------------- | ---------------------------- |
| P0       | document-manager.tsx        | ~~Add Loader2 import~~ ✅    |
| P1       | status-badge.tsx            | Add processing sub-states    |
| P1       | pipeline-status-dialog.tsx  | Enhance with per-doc details |
| P1       | reprocess-failed-button.tsx | Improve error feedback       |
| P2       | edgequake.ts (API)          | Add progress streaming       |
| P2       | documents.rs                | Emit processing stage events |
| P3       | workspaces.rs               | Improve rebuild progress     |

---

## Next Step

Proceed to **Orient** phase to analyze gaps and define solutions.
