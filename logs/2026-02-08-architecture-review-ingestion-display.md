# Architecture Review: Ingestion + Display System

**Date:** February 8, 2026  
**Scope:** Full-stack document ingestion pipeline + UX display layer  
**Methodology:** SRP, DRY, Reliability Analysis

---

## Executive Summary

### System Health: 🟡 **IMPROVEMENT NEEDED**

**Strengths:**

- ✅ Well-documented code with traceability (FEAT/UC/BR annotations)
- ✅ Comprehensive error types and structured error handling
- ✅ WebSocket-based real-time progress tracking (OODA-42)
- ✅ Per-workspace vector storage isolation (SPEC-033)
- ✅ Resilient chunk-level extraction with retry logic

**Critical Issues Found:**

- ⚠️ **5 SRP Violations** - Mixed concerns in handlers and components
- ⚠️ **8 DRY Violations** - Duplicated logic across backend/frontend
- ⚠️ **3 Silent Failure Modes** - Error states not surfaced to UI
- ⚠️ **2 Data Consistency Risks** - Race conditions in upload/reprocess

---

## 1. Architecture Overview

### 1.1 Backend Architecture (Rust)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         INGESTION PIPELINE                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Upload Handler (documents.rs)                                     │
│       │                                                             │
│       ├─ Validate Content (file_validation)                        │
│       ├─ Compute SHA-256 Hash (ContentHasher)                      │
│       ├─ Check Duplicates (KV Storage)                             │
│       ├─ Store Metadata + Content (KV Storage)                     │
│       └─ Spawn Async Task (TaskManager)                            │
│                │                                                    │
│                ▼                                                    │
│  Pipeline Executor (pipeline.rs)                                   │
│       │                                                             │
│       ├─ PDF Extraction (edgequake-pdf)                            │
│       │     └─ pdfium::extract_text_with_metadata()                │
│       │                                                             │
│       ├─ Chunking (chunker.rs)                                     │
│       │     └─ Split into 1200-token chunks with 100-token overlap │
│       │                                                             │
│       ├─ Entity Extraction (extractor.rs)                          │
│       │     ├─ LLM call (OpenAI/Ollama/LMStudio)                   │
│       │     ├─ Retry logic (3 attempts, exponential backoff)       │
│       │     └─ Cache results (MemoryLLMCache)                      │
│       │                                                             │
│       ├─ Graph Merging (merger.rs)                                 │
│       │     └─ Deduplicate entities + relationships                │
│       │                                                             │
│       └─ Embedding (LLM Provider)                                  │
│             └─ Generate vectors for chunks + entities              │
│                                                                     │
│  Progress Tracking (progress.rs)                                   │
│       │                                                             │
│       └─ ProgressTracker::send_progress()                          │
│             └─ WebSocket broadcast to frontend                     │
│                                                                     │
│  Error Handling (error.rs)                                         │
│       │                                                             │
│       ├─ PipelineError (structured error types)                    │
│       ├─ ApiError (HTTP error responses)                           │
│       └─ IngestionError (WebSocket error events)                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Frontend Architecture (TypeScript/React)

```
┌─────────────────────────────────────────────────────────────────────┐
│                           DISPLAY LAYER                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  DocumentManager (document-manager.tsx) - 1822 lines               │
│       │                                                             │
│       ├─ Upload UI (drag-and-drop, file selection)                 │
│       ├─ Document List (paginated table with status badges)        │
│       ├─ Batch Operations (delete, reprocess, cancel)              │
│       ├─ Filter/Sort Controls (status, date, title)                │
│       └─ Detail Panel (side-by-side PDF + Markdown viewer)         │
│             │                                                       │
│             ├─ EnhancedStatusBadge (status display with errors)    │
│             ├─ BatchProgressCard (removed in OODA-08)              │
│             └─ PipelineStatusDialog (active tasks view)            │
│                                                                     │
│  WebSocketProvider (websocket-provider.tsx)                        │
│       │                                                             │
│       ├─ Connection Management (auto-reconnect)                    │
│       ├─ Event Subscriptions (progress, pdf_progress, snapshot)    │
│       ├─ Error Handling (ingestion_failed events)                  │
│       └─ Toast Notifications (user feedback)                       │
│                                                                     │
│  IngestionStore (use-ingestion-store.ts) - Zustand                 │
│       │                                                             │
│       ├─ Tracks Map<track_id, IngestionProgress>                   │
│       ├─ Stage Progress (6 stages: preprocess → index)             │
│       ├─ Overall Progress (percentage calculation)                 │
│       └─ Error Tracking (failed jobs map)                          │
│                                                                     │
│  ReactQuery (tanstack/react-query)                                 │
│       │                                                             │
│       ├─ Documents Query (paginated, filtered)                     │
│       ├─ Cache Invalidation (on WebSocket events)                  │
│       └─ Optimistic Updates (instant UI feedback)                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Data Flow Diagram

```
User Upload
    │
    ├──► Frontend: DocumentManager.handleFilesUpload()
    │        │
    │        ├─ Generate track_id
    │        ├─ Call uploadPdfDocument(file, track_id)
    │        └─ Subscribe to WebSocket(track_id)
    │
    ├──► Backend: POST /api/v1/documents
    │        │
    │        ├─ Validate file (size, format)
    │        ├─ Compute SHA-256 hash
    │        ├─ Check duplicate (KV storage)
    │        │    ├─ Exists + completed? → Return 409
    │        │    └─ Exists + failed? → Delete old data
    │        │
    │        ├─ Store metadata (status: processing)
    │        ├─ Store content (raw PDF bytes)
    │        └─ Spawn async task
    │              │
    │              └──► TaskManager::spawn_ingestion_task()
    │                        │
    │                        ├─ Initialize ProgressTracker
    │                        │
    │                        ├─ Stage 1: PDF Extraction
    │                        │    ├─ Send WebSocket: pdf_progress
    │                        │    └─ Extract text + metadata
    │                        │
    │                        ├─ Stage 2: Chunking
    │                        │    ├─ Send WebSocket: stage_started
    │                        │    └─ Split into 1200-token chunks
    │                        │
    │                        ├─ Stage 3: Entity Extraction
    │                        │    ├─ Send WebSocket: stage_progress
    │                        │    ├─ LLM call per chunk (parallel)
    │                        │    └─ Retry failed chunks (3x)
    │                        │
    │                        ├─ Stage 4: Graph Merging
    │                        │    └─ Deduplicate entities
    │                        │
    │                        ├─ Stage 5: Embedding
    │                        │    └─ Generate vectors
    │                        │
    │                        └─ Stage 6: Indexing
    │                             └─ Store in vector DB
    │
    ├──► WebSocket: Progress Events
    │        │
    │        ├─ stage_started { stage, track_id }
    │        ├─ stage_progress { stage, progress, track_id }
    │        ├─ stage_completed { stage, result, track_id }
    │        ├─ ingestion_completed { document_id, track_id }
    │        └─ ingestion_failed { error, track_id, stage }
    │
    └──► Frontend: WebSocket Event Handlers
             │
             ├─ WebSocketProvider.handleMessage()
             │    ├─ Update IngestionStore (tracks Map)
             │    ├─ Show toast notification (errors)
             │    └─ Invalidate ReactQuery cache
             │
             └─ DocumentManager re-renders
                  └─ EnhancedStatusBadge updates
```

---

## 2. Single Responsibility Principle (SRP) Violations

### **Violation #1: DocumentManager Component (1822 lines)**

**Location:** `edgequake_webui/src/components/documents/document-manager.tsx`

**Issue:**  
Single component handles SEVEN distinct responsibilities:

1. Upload UI (file dropzone, progress)
2. Document list (table, pagination)
3. Batch operations (delete, reprocess, cancel)
4. Filter/sort state (status, date, title)
5. Detail panel (PDF + Markdown viewer)
6. WebSocket subscription management
7. Stuck document detection

**Code Evidence:**

```typescript
// Lines 101-1822 in document-manager.tsx
export default function DocumentManager() {
  // Upload state (7 variables)
  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  // List state (5 variables)
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [statusFilter, setStatusFilter] = useState<DocStatus>("all");

  // Detail panel state (2 variables)
  const [viewerPdfId, setViewerPdfId] = useState<string | null>(null);

  // WebSocket logic (40 lines)
  useEffect(() => {
    /* subscribe to WebSocket */
  }, [connected, data]);

  // Stuck document detection (30 lines)
  useEffect(() => {
    /* detect stuck docs */
  }, [data]);

  // Upload handler (150 lines)
  const handleFilesUpload = useCallback(async (files: File[]) => {
    /* complex upload logic */
  }, []);

  // 1500+ lines of JSX...
}
```

**Impact:**

- **Maintainability**: Changes to upload logic risk breaking list display
- **Testability**: Impossible to unit test individual features in isolation
- **Performance**: Component re-renders on ANY state change
- **Code Review**: 1822-line files are difficult to review effectively

**Recommended Fix:**

```typescript
// Split into 6 focused components:
DocumentManager/
  ├─ DocumentUploadZone.tsx      (upload UI + progress)
  ├─ DocumentList.tsx             (table + pagination)
  ├─ DocumentFilters.tsx          (status/sort controls)
  ├─ DocumentBatchActions.tsx     (multi-select operations)
  ├─ DocumentDetailPanel.tsx      (PDF + Markdown viewer)
  └─ hooks/
      ├─ useDocumentWebSocket.ts  (WebSocket subscription)
      └─ useStuckDetection.ts     (stuck document monitoring)
```

---

### **Violation #2: Upload Handler in documents.rs**

**Location:** `edgequake/crates/edgequake-api/src/handlers/documents.rs:600-900`

**Issue:**  
`upload_document()` function handles SIX responsibilities:

1. HTTP request parsing (multipart form data)
2. File validation (size, format)
3. Content hashing (SHA-256 computation)
4. Duplicate detection (KV storage lookup)
5. Metadata persistence (KV storage writes)
6. Task spawning (async pipeline orchestration)

**Code Evidence:**

```rust
// Lines 600-900 in documents.rs
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    tenant: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<Json<UploadDocumentResponse>> {
    // CONCERN 1: Multipart parsing (50 lines)
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("file") => { /* parse file */ },
            Some("track_id") => { /* parse track_id */ },
            _ => continue,
        }
    }

    // CONCERN 2: File validation (30 lines)
    validate_file(&filename, &content, &state.config)?;

    // CONCERN 3: Content hashing (10 lines)
    let content_hash = ContentHasher::compute_hash(&content);

    // CONCERN 4: Duplicate detection (40 lines)
    let existing_docs = search_documents_by_hash(&state, &content_hash).await?;
    if !existing_docs.is_empty() {
        // Complex duplicate handling logic...
    }

    // CONCERN 5: Metadata persistence (30 lines)
    let metadata = DocumentMetadata { /* ... */ };
    state.kv_storage.set(&metadata_key, &serde_json::to_value(&metadata)?).await?;

    // CONCERN 6: Task spawning (40 lines)
    let task_id = state.task_manager.spawn_ingestion_task(/* ... */).await?;

    // Return response
    Ok(Json(UploadDocumentResponse { /* ... */ }))
}
```

**Impact:**

- **Testability**: Cannot unit test duplicate detection without full HTTP context
- **Reusability**: Duplicate logic cannot be reused for batch uploads
- **Error Handling**: Complex error recovery logic mixed with business logic

**Recommended Fix:**

```rust
// Refactor into focused services:
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    tenant: TenantContext,
    multipart: Multipart,
) -> ApiResult<Json<UploadDocumentResponse>> {
    // STEP 1: Parse request (single concern)
    let upload_req = parse_multipart_upload(multipart).await?;

    // STEP 2: Delegate to service layer
    let result = state
        .document_service
        .handle_upload(upload_req, &tenant)
        .await?;

    Ok(Json(result))
}

// New service layer with focused methods:
impl DocumentService {
    async fn handle_upload(&self, req: UploadRequest, tenant: &TenantContext) -> Result<UploadResponse> {
        // Validate
        self.validator.validate_file(&req.filename, &req.content)?;

        // Check duplicates
        let hash = self.hasher.compute_hash(&req.content);
        self.handle_duplicate_if_exists(&hash, tenant).await?;

        // Persist
        let doc_id = self.storage.store_document(req, tenant).await?;

        // Spawn task
        let task_id = self.task_mgr.spawn_ingestion(doc_id, req.track_id).await?;

        Ok(UploadResponse { doc_id, task_id })
    }
}
```

---

### **Violation #3: WebSocketProvider Mixing Concerns**

**Location:** `edgequake_webui/src/providers/websocket-provider.tsx`

**Issue:**  
`WebSocketProvider` handles THREE distinct responsibilities:

1. WebSocket connection management (connect, disconnect, reconnect)
2. Message routing (progress, pdf_progress, status_snapshot)
3. UI feedback (toast notifications, error formatting)

**Code Evidence:**

```typescript
// Lines 70-120 in websocket-provider.tsx
const handleMessage = useCallback(
  (message: WebSocketProgressMessage) => {
    // CONCERN 1: Message parsing
    if (message.type === "ingestion_failed") {
      const failedEvent = message as IngestionFailedEvent;

      // CONCERN 2: Logging
      console.error("[WebSocket] Ingestion failed:", failedEvent);

      // CONCERN 3: UI feedback (toast notification)
      toast.error(`Document processing failed: ${failedEvent.error.message}`, {
        duration: 10000,
      });
    }

    // CONCERN 4: State management
    updateFromMessage(message);
    updateIngestionCost(costMessage);
  },
  [updateFromMessage, updateIngestionCost],
);
```

**Impact:**

- **Testability**: Cannot test message routing without mocking toast library
- **Reusability**: UI feedback logic coupled to WebSocket provider
- **Separation of Concerns**: Infrastructure (WebSocket) mixed with presentation (toast)

**Recommended Fix:**

```typescript
// Split into focused hooks:
// 1. Connection management
export function useWebSocketConnection() {
  const [connected, setConnected] = useState(false);
  // ... connection logic only
  return { connected, connect, disconnect };
}

// 2. Message routing
export function useWebSocketEvents(handler: MessageHandler) {
  const { connected } = useWebSocketConnection();
  // ... event subscription logic only
}

// 3. UI feedback (separate component)
export function IngestionErrorToasts() {
  const failedJobs = useIngestionStore((s) => s.failedJobs);

  useEffect(() => {
    failedJobs.forEach((job) => {
      toast.error(`Processing failed: ${job.error.message}`);
    });
  }, [failedJobs]);

  return null;
}
```

---

### **Violation #4: cleanup_document_graph_data Function**

**Location:** `edgequake/crates/edgequake-api/src/handlers/documents.rs:280-450`

**Issue:**  
Single function handles FOUR distinct responsibilities:

1. Graph node cleanup (remove document from source_ids)
2. Graph edge cleanup (handle orphaned relationships)
3. Vector storage cleanup (delete entity embeddings)
4. Statistics tracking (count removed/updated entities)

**Code Evidence:**

```rust
// Lines 280-450 in documents.rs
async fn cleanup_document_graph_data(
    document_id: &str,
    graph_storage: &Arc<dyn GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
) -> Result<CleanupStats, ApiError> {
    let mut stats = CleanupStats::default();

    // CONCERN 1: Process graph nodes (50 lines)
    let all_nodes = graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        // Remove document sources...
        // Delete nodes with empty sources...
    }

    // CONCERN 2: Process graph edges (50 lines)
    let all_edges = graph_storage.get_all_edges().await?;
    for edge in all_edges {
        // Check orphaned edges...
        // Update remaining sources...
    }

    // CONCERN 3: Delete embeddings (20 lines)
    if let Some(vs) = vector_storage {
        vs.delete_entity(&node.id).await;
    }

    // CONCERN 4: Track statistics (10 lines)
    stats.entities_removed += 1;
    stats.relationships_updated += 1;

    Ok(stats)
}
```

**Impact:**

- **Testability**: Cannot test node cleanup independently from edge cleanup
- **Performance**: All-nodes scan even when only few nodes affected
- **Reusability**: Cannot reuse node cleanup logic without edge logic

**Recommended Fix:**

```rust
// Split into focused functions:
struct GraphCleaner {
    graph: Arc<dyn GraphStorage>,
    vector: Option<Arc<dyn VectorStorage>>,
}

impl GraphCleaner {
    async fn cleanup_nodes(&self, doc_id: &str) -> Result<NodeCleanupStats> {
        // Only node cleanup logic
    }

    async fn cleanup_edges(&self, doc_id: &str, deleted_nodes: &HashSet<String>) -> Result<EdgeCleanupStats> {
        // Only edge cleanup logic
    }

    async fn cleanup_embeddings(&self, entity_ids: &[String]) -> Result<usize> {
        // Only embedding deletion
    }
}

// Caller orchestrates:
async fn cleanup_document_graph_data(..) -> Result<CleanupStats> {
    let cleaner = GraphCleaner { graph_storage, vector_storage };

    let node_stats = cleaner.cleanup_nodes(document_id).await?;
    let edge_stats = cleaner.cleanup_edges(document_id, &node_stats.deleted_ids).await?;
    let embeddings_deleted = cleaner.cleanup_embeddings(&node_stats.deleted_ids).await?;

    Ok(CleanupStats::combine(node_stats, edge_stats, embeddings_deleted))
}
```

---

### **Violation #5: Pipeline::process() Orchestration**

**Location:** `edgequake/crates/edgequake-pipeline/src/pipeline.rs:500-1200`

**Issue:**  
Single `process()` method handles SIX pipeline stages + error handling + progress tracking:

**Code Evidence:**

```rust
// Lines 500-1200 in pipeline.rs
pub async fn process<F>(
    &self,
    content: &str,
    document_id: &str,
    progress_callback: Option<F>,
) -> Result<ProcessingResult>
where F: Fn(ChunkProgressUpdate) + Send + Sync + 'static
{
    // CONCERN 1: Chunking (100 lines)
    let chunks = self.chunker.chunk(content, document_id)?;

    // CONCERN 2: Extraction (300 lines with retry logic)
    let extractions = self.extract_entities_batched(&chunks, progress_callback).await?;

    // CONCERN 3: Merging (150 lines)
    let merged = self.merger.merge(&extractions)?;

    // CONCERN 4: Embedding (100 lines)
    let embeddings = self.generate_embeddings(&chunks, &merged).await?;

    // CONCERN 5: Statistics (50 lines)
    let stats = self.calculate_stats(&chunks, &extractions);

    // CONCERN 6: Lineage (50 lines)
    let lineage = self.build_lineage(document_id, &chunks, &extractions);

    Ok(ProcessingResult { /* ... */ })
}
```

**Recommended Fix:**

```rust
// Split into stage-specific methods:
impl Pipeline {
    pub async fn process(&self, content: &str, doc_id: &str) -> Result<ProcessingResult> {
        let context = ProcessingContext::new(doc_id);

        // Stage 1: Chunking
        let chunks = self.execute_chunking_stage(content, &context).await?;

        // Stage 2: Extraction
        let extractions = self.execute_extraction_stage(&chunks, &context).await?;

        // Stage 3: Merging
        let graph = self.execute_merging_stage(&extractions, &context).await?;

        // Stage 4: Embedding
        let embeddings = self.execute_embedding_stage(&chunks, &graph, &context).await?;

        // Stage 5: Build result
        Ok(ProcessingResult::from_stages(chunks, extractions, graph, embeddings, context.stats))
    }

    async fn execute_chunking_stage(&self, content: &str, ctx: &ProcessingContext) -> Result<Vec<TextChunk>> {
        ctx.progress.start_stage("chunking");
        let result = self.chunker.chunk(content, ctx.document_id)?;
        ctx.progress.complete_stage("chunking", result.len());
        Ok(result)
    }

    // Similar focused methods for other stages...
}
```

---

## 3. Don't Repeat Yourself (DRY) Violations

### **Duplication #1: Document Status Filtering**

**Duplicated In:**

- `edgequake/crates/edgequake-api/src/handlers/documents.rs` (backend filtering)
- `edgequake_webui/src/components/documents/document-manager.tsx` (frontend filtering)
- `edgequake_webui/src/types/index.ts` (type definitions)

**Code Evidence:**

```rust
// Backend (documents.rs:1200-1250)
let filtered_docs: Vec<DocumentMetadataResponse> = all_docs
    .into_iter()
    .filter(|doc| {
        // Filter by status
        if let Some(status) = &query.status {
            if doc.status != *status {
                return false;
            }
        }

        // Filter by tenant
        if doc.tenant_id != tenant.tenant_id {
            return false;
        }

        // Filter by workspace
        if doc.workspace_id != tenant.workspace_id {
            return false;
        }

        true
    })
    .collect();
```

```typescript
// Frontend (document-manager.tsx:800-850)
const filteredDocs =
  data?.items?.filter((doc: Document) => {
    // Status filter
    if (statusFilter !== "all" && doc.status !== statusFilter) {
      return false;
    }

    // Search filter
    if (
      searchQuery &&
      !doc.title.toLowerCase().includes(searchQuery.toLowerCase())
    ) {
      return false;
    }

    return true;
  }) || [];
```

**Impact:**

- **Inconsistency**: Backend and frontend may filter differently
- **Maintainability**: Status filter changes require updates in 3 places
- **Performance**: Double filtering (backend + frontend) wastes resources

**Recommended Fix:**

```typescript
// Shared filtering specification (JSON Schema or TypeScript interface)
// File: shared/document-filters.schema.json
{
  "DocumentFilters": {
    "status": { "type": "string", "enum": ["processing", "completed", "failed", "pending"] },
    "search": { "type": "string", "maxLength": 500 },
    "dateFrom": { "type": "string", "format": "date-time" },
    "dateTo": { "type": "string", "format": "date-time" }
  }
}

// Backend: Generate filter code from schema
// Frontend: Use same schema for validation
// Result: Single source of truth for filter logic
```

---

### **Duplication #2: Status Badge Logic**

**Duplicated In:**

- `edgequake_webui/src/components/documents/enhanced-status-badge.tsx` (status priority + transitions)
- `edgequake_webui/src/components/documents/document-manager.tsx` (isProcessingStatus helper)
- Backend status updates (multiple handlers)

**Code Evidence:**

```typescript
// enhanced-status-badge.tsx (lines 100-150)
const displayStatus = useMemo(() => {
  // Priority 0: Error message
  if (document.error_message?.trim()) {
    return "failed";
  }

  // Smart transitions
  const baseStatus = document.status as IngestionStage;
  const msg = (
    track?.progress?.latest_message ||
    document.stage_message ||
    ""
  ).toLowerCase();

  if (baseStatus === "converting" && msg.includes("complete")) {
    return "chunking"; // Show next stage
  }

  if (baseStatus === "chunking" && msg.includes("complete")) {
    return "extracting";
  }

  return baseStatus;
}, [document, track]);
```

```typescript
// document-manager.tsx (lines 150-170)
function isProcessingStatus(status: string | null | undefined): boolean {
  if (!status) return false;
  return [
    "processing",
    "chunking",
    "extracting",
    "embedding",
    "indexing",
  ].includes(status);
}
```

**Impact:**

- **Inconsistency**: Status transition logic differs between components
- **Fragility**: Adding new status requires updates in multiple locations
- **Testing**: Must test same logic in multiple test suites

**Recommended Fix:**

```typescript
// Centralized status state machine
// File: lib/status-machine.ts
export const STATUS_MACHINE = {
  states: {
    pending: { next: ["preprocessing"], canCancel: true },
    preprocessing: { next: ["converting"], canCancel: true },
    converting: { next: ["chunking"], canCancel: true },
    chunking: { next: ["extracting"], canCancel: false },
    extracting: { next: ["merging"], canCancel: false },
    merging: { next: ["embedding"], canCancel: false },
    embedding: { next: ["indexing"], canCancel: false },
    indexing: { next: ["completed"], canCancel: false },
    completed: { next: [], canCancel: false },
    failed: { next: ["pending"], canCancel: false },
  },

  getNextStage(current: string, message?: string): string {
    if (message?.includes("complete")) {
      return this.states[current]?.next[0] || current;
    }
    return current;
  },

  isProcessing(status: string): boolean {
    return [
      "preprocessing",
      "converting",
      "chunking",
      "extracting",
      "merging",
      "embedding",
      "indexing",
    ].includes(status);
  },

  canTransition(from: string, to: string): boolean {
    return this.states[from]?.next.includes(to) || false;
  },
};

// Use in components:
const displayStatus = STATUS_MACHINE.getNextStage(
  document.status,
  track?.progress?.latest_message,
);
const isProcessing = STATUS_MACHINE.isProcessing(document.status);
```

---

### **Duplication #3: WebSocket Event Handling**

**Duplicated In:**

- `edgequake_webui/src/providers/websocket-provider.tsx` (event parsing + store updates)
- `edgequake_webui/src/stores/use-ingestion-store.ts` (event handling logic)
- `edgequake_webui/src/lib/websocket.ts` (low-level message routing)

**Code Evidence:**

```typescript
// websocket-provider.tsx (lines 70-120)
const handleMessage = useCallback(
  (message: WebSocketProgressMessage) => {
    if (message.type === "ingestion_failed") {
      const failedEvent = message as IngestionFailedEvent;
      console.error("[WebSocket] Ingestion failed:", failedEvent);
      toast.error(`Document processing failed: ${failedEvent.error.message}`);
    } else if (message.type === "ingestion_completed") {
      console.log("[WebSocket] Ingestion completed:", message);
    }

    updateFromMessage(message);
  },
  [updateFromMessage],
);
```

```typescript
// use-ingestion-store.ts (lines 200-300)
updateFromMessage: (message: WebSocketProgressMessage) => {
  if (message.type === "ingestion_started") {
    return handleIngestionStarted(state, message);
  } else if (message.type === "stage_started") {
    return handleStageStarted(state, message);
  } else if (message.type === "stage_progress") {
    return handleStageProgress(state, message);
  }
  // ... more conditionals
};
```

**Recommended Fix:**

```typescript
// Event-driven architecture with type-safe handlers
// File: lib/websocket-events.ts
type EventHandler<T extends WebSocketProgressMessage> = (
  state: IngestionState,
  event: T,
) => IngestionState;

const EVENT_HANDLERS: Record<
  WebSocketProgressMessage["type"],
  EventHandler<any>
> = {
  ingestion_started: handleIngestionStarted,
  stage_started: handleStageStarted,
  stage_progress: handleStageProgress,
  stage_completed: handleStageCompleted,
  ingestion_completed: handleIngestionCompleted,
  ingestion_failed: handleIngestionFailed,
};

// Single dispatch function
export function dispatchWebSocketEvent(
  state: IngestionState,
  message: WebSocketProgressMessage,
): IngestionState {
  const handler = EVENT_HANDLERS[message.type];
  if (!handler) {
    console.warn("[WebSocket] Unknown event type:", message.type);
    return state;
  }
  return handler(state, message);
}

// Use in provider:
const handleMessage = useCallback((message: WebSocketProgressMessage) => {
  const newState = dispatchWebSocketEvent(ingestionStore.getState(), message);
  ingestionStore.setState(newState);
}, []);
```

---

### **Duplication #4: Error Formatting**

**Duplicated In:**

- Backend: `edgequake/crates/edgequake-pipeline/src/error.rs` (PipelineError formatting)
- Frontend: `edgequake_webui/src/components/documents/enhanced-status-badge.tsx` (error display)
- Frontend: `edgequake_webui/src/providers/websocket-provider.tsx` (toast error formatting)

**Code Evidence:**

```rust
// Backend error.rs (lines 50-100)
impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Extraction(msg) => write!(f, "Extraction failed: {}", msg),
            Self::LLMTimeout(secs) => write!(f, "LLM call timed out after {} seconds", secs),
            Self::ChunkProcessing { chunk_id, error } => {
                write!(f, "Chunk {} failed: {}", chunk_id, error)
            }
        }
    }
}
```

```typescript
// Frontend enhanced-status-badge.tsx (lines 200-220)
const progressMessage = useMemo(() => {
  if (document.error_message?.trim()) {
    return `Error: ${document.error_message}`;
  }
  // ... more formatting
}, [document]);
```

```typescript
// Frontend websocket-provider.tsx (lines 80-90)
toast.error(`Document processing failed: ${failedEvent.error.message}`, {
  description: `Stage: ${failedEvent.stage}`,
});
```

**Recommended Fix:**

```typescript
// Shared error specification (use existing error_codes from ingestion_types.rs)
// File: shared/error-formatter.ts
export interface ErrorDetails {
  code: string; // e.g., "EXTRACT_TIMEOUT", "CHUNK_FAILED"
  message: string;
  stage?: string;
  recoverable: boolean;
  userAction?: string; // "Click Retry" | "Check LLM service" | "Contact support"
}

export class ErrorFormatter {
  static formatForUI(error: ErrorDetails): string {
    const stageInfo = error.stage ? ` (Stage: ${error.stage})` : "";
    return `${error.message}${stageInfo}`;
  }

  static formatForLog(error: ErrorDetails): string {
    return `[${error.code}] ${error.message} | Recoverable: ${error.recoverable}`;
  }

  static getToastConfig(error: ErrorDetails) {
    return {
      duration: error.recoverable ? 5000 : 10000,
      action: error.recoverable
        ? { label: "Retry", onClick: () => {} }
        : undefined,
      description: error.userAction || error.stage,
    };
  }
}

// Use everywhere:
const errorDetails = ErrorFormatter.parseBackendError(failedEvent.error);
toast.error(
  ErrorFormatter.formatForUI(errorDetails),
  ErrorFormatter.getToastConfig(errorDetails),
);
```

---

### **Duplication #5: Workspace Storage Resolution**

**Duplicated In:**

- `documents.rs` (get_workspace_vector_storage_strict)
- `query.rs` (workspace storage lookup for queries)
- `costs.rs` (workspace filtering for cost aggregation)

**Code Evidence:**

```rust
// documents.rs (lines 75-130)
async fn get_workspace_vector_storage_strict(..) -> Result<Arc<dyn VectorStorage>> {
    let workspace_uuid = Uuid::parse_str(workspace_id)?;
    let workspace = state.workspace_service.get_workspace(workspace_uuid).await?;
    let config = WorkspaceVectorConfig {
        workspace_id: workspace_uuid,
        dimension: workspace.embedding_dimension,
        namespace: "default".to_string(),
    };
    state.vector_registry.get_or_create(config).await
}
```

```rust
// query.rs (similar logic 40 lines)
// costs.rs (similar logic 35 lines)
```

**Recommended Fix:**

```rust
// Centralized workspace resolver service
pub struct WorkspaceResolver {
    workspace_service: Arc<WorkspaceService>,
    vector_registry: Arc<VectorRegistry>,
}

impl WorkspaceResolver {
    pub async fn resolve_workspace(&self, id: &str) -> Result<Workspace> {
        let uuid = Uuid::parse_str(id).map_err(|e| ApiError::BadRequest(...))?;
        self.workspace_service.get_workspace(uuid).await?
            .ok_or_else(|| ApiError::NotFound(...))
    }

    pub async fn get_vector_storage(&self, workspace_id: &str) -> Result<Arc<dyn VectorStorage>> {
        let workspace = self.resolve_workspace(workspace_id).await?;
        let config = WorkspaceVectorConfig::from_workspace(&workspace);
        self.vector_registry.get_or_create(config).await
    }
}

// Use in handlers:
let vector_storage = state.workspace_resolver.get_vector_storage(&workspace_id).await?;
```

---

### **Duplication #6: Progress Calculation**

**Duplicated In:**

- Backend: `edgequake/crates/edgequake-pipeline/src/progress.rs` (ProgressTracker::overall_progress)
- Frontend: `use-ingestion-store.ts` (calculateOverallProgress helper)

**Code Evidence:**

```rust
// Backend progress.rs (lines 400-420)
pub fn overall_progress(&self) -> f64 {
    let stages = self.stages.read();
    let total_weight: f64 = stages.values().map(|s| s.weight).sum();
    let completed_weight: f64 = stages.values()
        .map(|s| (s.progress / 100.0) * s.weight)
        .sum();
    (completed_weight / total_weight) * 100.0
}
```

```typescript
// Frontend use-ingestion-store.ts (lines 350-380)
function calculateOverallProgress(stages: StageProgress[]): number {
  const weights = {
    preprocessing: 5,
    chunking: 10,
    extracting: 40,
    merging: 20,
    embedding: 20,
    indexing: 5,
  };

  let totalWeight = 0;
  let completedWeight = 0;
  stages.forEach((stage) => {
    const weight = weights[stage.stage] || 10;
    totalWeight += weight;
    completedWeight += (stage.progress / 100) * weight;
  });

  return totalWeight > 0 ? (completedWeight / totalWeight) * 100 : 0;
}
```

**Impact:**

- **Inconsistency**: Backend and frontend may show different progress percentages
- **Fragility**: Changing stage weights requires updates in 2 places

**Recommended Fix:**

```rust
// Backend: Export stage weights in API response
#[derive(Serialize)]
pub struct ProgressConfig {
    pub stage_weights: HashMap<String, f64>,
}

impl PipelineConfig {
    pub fn get_progress_config() -> ProgressConfig {
        ProgressConfig {
            stage_weights: hashmap! {
                "preprocessing".to_string() => 5.0,
                "chunking".to_string() => 10.0,
                "extracting".to_string() => 40.0,
                "merging".to_string() => 20.0,
                "embedding".to_string() => 20.0,
                "indexing".to_string() => 5.0,
            },
        }
    }
}

// Frontend: Fetch weights from backend
const { data: progressConfig } = useQuery(['progress-config'], () =>
  fetchProgressConfig()
);

function calculateOverallProgress(stages: StageProgress[], config: ProgressConfig): number {
  // Use backend-provided weights
  const weights = config.stage_weights;
  // ... same calculation logic
}
```

---

### **Duplication #7: Track ID Generation**

**Duplicated In:**

- Frontend: `document-manager.tsx` (upload handler generates track_id)
- Backend: `documents.rs` (generates track_id if not provided)
- Task manager (generates task_id)

**Code Evidence:**

```typescript
// Frontend document-manager.tsx (lines 330-340)
const trackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
```

```rust
// Backend documents.rs (lines 650-660)
let track_id = track_id.unwrap_or_else(|| {
    format!("track-{}-{}", Uuid::new_v4(), Utc::now().timestamp_millis())
});
```

**Impact:**

- **Inconsistency**: Different ID formats make debugging harder
- **Collision Risk**: Weak random generation (Math.random) not cryptographically secure

**Recommended Fix:**

```typescript
// Shared ID generation utility
// File: shared/id-generator.ts
export class IdGenerator {
  static generateTrackId(): string {
    // Use UUID v7 (timestamp-based, sortable, globally unique)
    return `track-${uuidv7()}`;
  }

  static generateTaskId(): string {
    return `task-${uuidv7()}`;
  }

  static generateDocumentId(): string {
    return `doc-${uuidv7()}`;
  }
}

// Frontend usage:
const trackId = IdGenerator.generateTrackId();

// Backend: Accept frontend-generated IDs (validate format)
let track_id = validate_track_id(provided_track_id)?;
```

---

### **Duplication #8: Chunk Prefix Matching**

**Duplicated In:**

- `cleanup_document_graph_data` (documents.rs)
- `delete_document_for_reingestion` (documents.rs)
- Multiple storage adapters

**Code Evidence:**

```rust
// documents.rs (lines 340, 480, 620)
let chunk_prefix = format!("{}-chunk-", document_id); // Repeated 3 times

// Usage 1 (line 340):
let remaining_sources: Vec<String> = sources.iter()
    .filter(|s| !s.starts_with(&chunk_prefix))
    .cloned()
    .collect();

// Usage 2 (line 480):
let chunk_ids: Vec<String> = keys.iter()
    .filter(|k| k.starts_with(&chunk_prefix))
    .cloned()
    .collect();

// Usage 3 (line 620):
if source.starts_with(&chunk_prefix) { /* ... */ }
```

**Recommended Fix:**

```rust
// Centralized ID convention utilities
pub struct DocumentIdConventions;

impl DocumentIdConventions {
    pub fn chunk_prefix(document_id: &str) -> String {
        format!("{}-chunk-", document_id)
    }

    pub fn chunk_id(document_id: &str, chunk_index: usize) -> String {
        format!("{}-chunk-{}", document_id, chunk_index)
    }

    pub fn metadata_key(document_id: &str) -> String {
        format!("{}-metadata", document_id)
    }

    pub fn content_key(document_id: &str) -> String {
        format!("{}-content", document_id)
    }

    pub fn is_chunk_source(&self, source: &str, document_id: &str) -> bool {
        source.starts_with(&Self::chunk_prefix(document_id))
    }
}

// Usage:
let chunk_prefix = DocumentIdConventions::chunk_prefix(document_id);
let is_chunk = DocumentIdConventions::is_chunk_source(&source, document_id);
```

---

## 4. Reliability Issues

### **Issue #1: Silent WebSocket Disconnection**

**Location:** Frontend WebSocket handling

**Problem:**  
WebSocket disconnects are logged but not surfaced to user. Users may think their documents are processing when connection is dead.

**Code Evidence:**

```typescript
// websocket-provider.tsx (lines 130-140)
const unsubDisconnected = client.on("disconnected", () => {
  connectedRef.current = false;
  setWsConnected(false);
  // NO USER NOTIFICATION - Silent failure
});
```

**Impact:**

- Users upload documents, see "Processing..." status
- WebSocket disconnects silently
- Status badge shows stale "Processing" state indefinitely
- User doesn't know connection is dead until they refresh page

**Recommended Fix:**

```typescript
// Add connection status banner
const unsubDisconnected = client.on("disconnected", () => {
  connectedRef.current = false;
  setWsConnected(false);

  // Show persistent banner (not toast - stays visible)
  toast.warning("Connection lost - progress updates paused", {
    id: "websocket-disconnected",
    duration: Infinity,
    action: {
      label: "Reconnect",
      onClick: () => client.connect(),
    },
  });
});

// Clear banner on reconnect
const unsubConnected = client.on("connected", () => {
  connectedRef.current = true;
  setWsConnected(true);
  toast.dismiss("websocket-disconnected");
  toast.success("Connection restored", { duration: 3000 });
});
```

---

### **Issue #2: Race Condition in Re-ingestion**

**Location:** `delete_document_for_reingestion` function

**Problem:**  
Function checks document status, then deletes data. But status can change between check and delete (TOCTOU vulnerability).

**Code Evidence:**

```rust
// documents.rs (lines 470-490)
async fn delete_document_for_reingestion(...) -> Result<bool> {
    // CHECK: Get document status
    let status = if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&metadata_key).await {
        metadata.get("status").and_then(|v| v.as_str()).map(|s| s.to_string())
    } else {
        "unknown".to_string()
    };

    // RACE WINDOW: Status can change here!
    // Another request could start processing this document

    // DROP: Delete without atomicity
    cleanup_document_graph_data(document_id, &state.graph_storage, Some(&workspace_vector_storage)).await?;
    state.kv_storage.delete(&keys_to_delete).await?;

    Ok(true)
}
```

**Impact:**

- User A: Uploads document → starts processing
- User B: Re-uploads same document → triggers re-ingestion
- User A's processing writes partial data while User B's cleanup is deleting
- Result: Corrupted graph state with orphaned entities

**Recommended Fix:**

```rust
// Use atomic operations with version checking
async fn delete_document_for_reingestion(...) -> Result<bool> {
    // Acquire distributed lock
    let lock = state.lock_manager.acquire_document_lock(document_id).await?;

    // Re-check status under lock
    let metadata = state.kv_storage.get_by_id(&metadata_key).await?
        .ok_or_else(|| ApiError::NotFound("Document not found"))?;

    let status = metadata.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

    // Fail if document is actively processing
    if status == "processing" || status == "pending" {
        return Err(ApiError::Conflict("Cannot re-ingest document that is currently processing"));
    }

    // Atomic cleanup
    cleanup_document_graph_data(document_id, &state.graph_storage, Some(&workspace_vector_storage)).await?;
    state.kv_storage.delete(&keys_to_delete).await?;

    // Release lock
    drop(lock);

    Ok(true)
}
```

---

### **Issue #3: Chunk Extraction Partial Failure Not Visible**

**Location:** Pipeline processing with chunk-level failures

**Problem:**  
Pipeline successfully processes 8/10 chunks, marks document as "completed", but user doesn't know 2 chunks failed.

**Code Evidence:**

```rust
// pipeline.rs (lines 800-850)
pub async fn extract_entities_batched(...) -> Result<Vec<ExtractionResult>> {
    let results = stream::iter(chunks)
        .map(|chunk| async move {
            match self.extract_with_retry(chunk).await {
                Ok(result) => result,
                Err(e) => {
                    // Log error but continue processing
                    tracing::warn!("Chunk {} failed: {}", chunk.id, e);
                    ExtractionResult::empty() // Return empty result
                }
            }
        })
        .buffer_unordered(self.config.max_concurrent_extractions)
        .collect::<Vec<_>>()
        .await;

    // ALL CHUNKS returned (some empty), marked as SUCCESS
    Ok(results)
}
```

**Impact:**

- Document shows status: "Completed" ✅
- User queries document: "0 Sources found" ❌
- User doesn't know extraction failed for 2/10 chunks
- No way to retry just the failed chunks

**Recommended Fix:**

```rust
// Return detailed success/failure breakdown
pub struct ExtractionStats {
    pub total_chunks: usize,
    pub successful_chunks: usize,
    pub failed_chunks: usize,
    pub chunk_errors: Vec<ChunkErrorInfo>, // New struct with chunk_id + error
}

pub async fn extract_entities_batched(...) -> Result<(Vec<ExtractionResult>, ExtractionStats)> {
    let mut stats = ExtractionStats::default();
    stats.total_chunks = chunks.len();

    let results: Vec<ExtractionResult> = stream::iter(chunks)
        .map(|chunk| async move {
            match self.extract_with_retry(chunk).await {
                Ok(result) => {
                    stats.successful_chunks += 1;
                    result
                }
                Err(e) => {
                    stats.failed_chunks += 1;
                    stats.chunk_errors.push(ChunkErrorInfo {
                        chunk_id: chunk.id.clone(),
                        error: e.to_string(),
                        retries_attempted: self.config.chunk_max_retries,
                    });
                    ExtractionResult::empty()
                }
            }
        })
        .buffer_unordered(self.config.max_concurrent_extractions)
        .collect()
        .await;

    // Set document status based on success rate
    if stats.failed_chunks == 0 {
        // Fully completed
    } else if stats.successful_chunks > 0 {
        // Partially completed (NEW STATUS)
        document_status = "partial_success";
        error_message = format!("{} of {} chunks failed extraction", stats.failed_chunks, stats.total_chunks);
    } else {
        // Fully failed
        document_status = "failed";
    }

    Ok((results, stats))
}
```

**Frontend Display:**

```typescript
// enhanced-status-badge.tsx
function EnhancedStatusBadge({ document }) {
  if (document.status === 'partial_success') {
    return (
      <Badge variant="warning">
        <AlertTriangle className="w-3 h-3 mr-1" />
        Partial ({document.successful_chunks}/{document.total_chunks} chunks)
      </Badge>
    );
  }
  // ... other statuses
}
```

---

## 5. Priority Recommendations

### **🔴 CRITICAL (Fix Immediately)**

1. **[RELIABILITY] Fix Race Condition in Re-ingestion** (Est: 4 hours)
   - Add distributed locking for document operations
   - Prevent concurrent upload + reprocess collisions
   - **Impact:** Prevents data corruption

2. **[RELIABILITY] Surface WebSocket Disconnection** (Est: 2 hours)
   - Add persistent connection status banner
   - Show "Connection Lost" warning to users
   - **Impact:** Users know when progress updates are stale

3. **[RELIABILITY] Expose Partial Extraction Failures** (Est: 6 hours)
   - Add "partial_success" document status
   - Show chunk success rate in UI
   - **Impact:** Users understand why queries return no results

---

### **🟡 HIGH PRIORITY (Fix This Sprint)**

4. **[SRP] Split DocumentManager into 6 Components** (Est: 12 hours)
   - Extract DocumentUploadZone, DocumentList, DocumentFilters, etc.
   - Create useDocumentWebSocket hook
   - **Impact:** Improves maintainability, testability, performance

5. **[DRY] Centralize Status Machine Logic** (Est: 4 hours)
   - Create STATUS_MACHINE with state transitions
   - Use in EnhancedStatusBadge, document-manager, backend
   - **Impact:** Single source of truth for status logic

6. **[SRP] Refactor upload_document Handler** (Est: 8 hours)
   - Extract DocumentService with focused methods
   - Separate concerns: validation, hashing, persistence, task spawning
   - **Impact:** Testable, reusable upload logic

---

### **🟢 MEDIUM PRIORITY (Fix Next Sprint)**

7. **[DRY] Centralize Error Formatting** (Est: 4 hours)
   - Use existing error_codes from ingestion_types.rs
   - Create ErrorFormatter utility
   - **Impact:** Consistent error messages across stack

8. **[DRY] Centralize Document ID Conventions** (Est: 2 hours)
   - Create DocumentIdConventions utility
   - Replace all chunk_prefix string formatting
   - **Impact:** Prevents ID format mismatches

9. **[SRP] Split cleanup_document_graph_data** (Est: 4 hours)
   - Extract GraphCleaner service
   - Separate node cleanup from edge cleanup
   - **Impact:** Performance optimization (targeted cleanup)

---

### **🔵 LOW PRIORITY (Technical Debt)**

10. **[DRY] Share Progress Calculation Logic** (Est: 3 hours)
    - Backend exports stage weights via API
    - Frontend fetches weights dynamically
    - **Impact:** Consistent progress percentages

11. **[DRY] Standardize Track ID Generation** (Est: 2 hours)
    - Use UUID v7 for sortable, collision-resistant IDs
    - Frontend and backend use same format
    - **Impact:** Better debugging, lower collision risk

12. **[SRP] Refactor Pipeline::process()** (Est: 8 hours)
    - Extract stage-specific methods (execute_chunking_stage, etc.)
    - Cleaner orchestration logic
    - **Impact:** Easier to add new pipeline stages

---

## 6. Testing Strategy

### **Unit Tests (Missing Coverage)**

```rust
// Backend: Test cleanup logic in isolation
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_cleanup_nodes_removes_document_sources() {
        let cleaner = GraphCleaner::new(mock_graph, mock_vector);
        let stats = cleaner.cleanup_nodes("doc-123").await.unwrap();
        assert_eq!(stats.entities_removed, 2);
        assert_eq!(stats.entities_updated, 3);
    }

    #[tokio::test]
    async fn test_cleanup_handles_orphaned_edges() {
        // Test edge cleanup after node deletion
    }
}
```

```typescript
// Frontend: Test status machine transitions
describe("STATUS_MACHINE", () => {
  it("should transition from converting to chunking when complete", () => {
    const next = STATUS_MACHINE.getNextStage(
      "converting",
      "PDF conversion complete",
    );
    expect(next).toBe("chunking");
  });

  it("should prevent transition to invalid state", () => {
    const canTransition = STATUS_MACHINE.canTransition("chunking", "completed");
    expect(canTransition).toBe(false);
  });
});
```

### **Integration Tests (E2E Scenarios)**

```typescript
// Test WebSocket disconnection handling
describe("WebSocket Reliability", () => {
  it("should show connection lost banner when disconnected", async () => {
    // Upload document
    // Disconnect WebSocket
    // Verify banner appears
    // Reconnect
    // Verify banner disappears
  });
});

// Test partial extraction failure
describe("Partial Extraction Failure", () => {
  it("should mark document as partial_success when 2/10 chunks fail", async () => {
    // Mock LLM to fail on 2 chunks
    // Upload document
    // Wait for completion
    // Verify status = 'partial_success'
    // Verify error_message shows chunk count
  });
});
```

---

## 7. Metrics & Monitoring

### **Reliability Metrics to Track**

```typescript
// Add to backend metrics
export const INGESTION_METRICS = {
  // Success rates
  ingestion_success_rate: new Gauge(
    "ingestion_success_rate",
    "Percentage of successful ingestions",
  ),
  chunk_extraction_success_rate: new Gauge(
    "chunk_extraction_success_rate",
    "Percentage of chunks extracted successfully",
  ),

  // Error rates
  websocket_disconnects: new Counter(
    "websocket_disconnects",
    "Number of WebSocket disconnections",
  ),
  race_condition_detected: new Counter(
    "race_condition_detected",
    "Re-ingestion race conditions prevented",
  ),

  // Performance
  extraction_latency_p99: new Histogram(
    "extraction_latency_p99",
    "P99 chunk extraction latency",
  ),
  partial_success_documents: new Counter(
    "partial_success_documents",
    "Documents with partial extraction failures",
  ),
};
```

### **Dashboard Queries**

```sql
-- Failed ingestions by stage
SELECT stage, COUNT(*) as failures
FROM ingestion_logs
WHERE status = 'failed'
  AND created_at > NOW() - INTERVAL '24 hours'
GROUP BY stage
ORDER BY failures DESC;

-- Documents with partial success
SELECT document_id, successful_chunks, failed_chunks
FROM document_processing_stats
WHERE failed_chunks > 0
  AND created_at > NOW() - INTERVAL '7 days'
ORDER BY failed_chunks DESC
LIMIT 20;

-- WebSocket disconnect frequency
SELECT DATE_TRUNC('hour', timestamp) as hour, COUNT(*) as disconnects
FROM websocket_events
WHERE event_type = 'disconnected'
  AND timestamp > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour DESC;
```

---

## 8. Implementation Roadmap

### **Week 1: Critical Reliability Fixes**

- [ ] Day 1-2: Fix race condition in re-ingestion (with locks)
- [ ] Day 2-3: Add WebSocket disconnection banner
- [ ] Day 3-5: Implement partial extraction failure visibility

### **Week 2: SRP Refactoring (Backend)**

- [ ] Day 1-2: Extract DocumentService from upload handler
- [ ] Day 3-4: Split cleanup_document_graph_data into GraphCleaner
- [ ] Day 5: Add unit tests for new services

### **Week 3: SRP Refactoring (Frontend)**

- [ ] Day 1-3: Split DocumentManager into 6 components
- [ ] Day 4: Create useDocumentWebSocket hook
- [ ] Day 5: Add component tests

### **Week 4: DRY Improvements**

- [ ] Day 1: Centralize status machine logic
- [ ] Day 2: Centralize error formatting
- [ ] Day 3: Standardize ID generation
- [ ] Day 4: Share progress calculation
- [ ] Day 5: Update documentation

---

## 9. Success Criteria

### **Quantitative Metrics**

- ✅ DocumentManager component: 1822 lines → <300 lines per component
- ✅ Upload handler: 300 lines → <100 lines (logic in service layer)
- ✅ Code duplication: 8 violations → 0 violations
- ✅ Test coverage: <40% → >80% for critical paths
- ✅ Chunk extraction success rate: 85% → 95%+ (visible failures)

### **Qualitative Improvements**

- ✅ Users see "Connection Lost" banner when WebSocket disconnects
- ✅ Users see "Partial Success (8/10 chunks)" status for failed extractions
- ✅ Developers can unit test upload logic without HTTP context
- ✅ Status transitions follow centralized state machine
- ✅ Error messages consistent between backend logs and frontend toasts

---

## Appendix: File-By-File Summary

### **Backend (Rust)**

| File           | Lines | Issues                                                                                   | Priority    |
| -------------- | ----- | ---------------------------------------------------------------------------------------- | ----------- |
| `documents.rs` | 4659  | SRP violation (upload handler), DRY (workspace resolution), Reliability (race condition) | 🔴 CRITICAL |
| `pipeline.rs`  | 2138  | SRP violation (process method), DRY (progress calculation)                               | 🟡 HIGH     |
| `progress.rs`  | 800   | DRY (progress calculation duplicated in frontend)                                        | 🟢 MEDIUM   |

### **Frontend (TypeScript/React)**

| File                        | Lines | Issues                                                             | Priority    |
| --------------------------- | ----- | ------------------------------------------------------------------ | ----------- |
| `document-manager.tsx`      | 1822  | SRP violation (7 responsibilities), DRY (status filtering)         | 🔴 CRITICAL |
| `websocket-provider.tsx`    | 220   | SRP violation (mixed concerns), Reliability (silent disconnection) | 🔴 CRITICAL |
| `enhanced-status-badge.tsx` | 300   | DRY (status logic duplicated)                                      | 🟡 HIGH     |
| `use-ingestion-store.ts`    | 744   | DRY (event handling, progress calculation)                         | 🟢 MEDIUM   |

---

**End of Report**
