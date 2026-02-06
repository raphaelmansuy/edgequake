# OODA Iteration 07 - Observe

**Date**: 2026-02-06
**Focus**: E2E Pipeline Verification and Document Visibility Investigation

## Observation Summary

This iteration focused on verifying the full E2E pipeline works and investigating an apparent document visibility issue reported during conversation summary recovery.

## System State Observed

### 1. Services Health

```
┌───────────────────────────────────────────────────────────────┐
│ Service          │ Status   │ Port  │ Notes                 │
├───────────────────────────────────────────────────────────────┤
│ Backend (Axum)   │ Healthy  │ 8080  │ PostgreSQL storage    │
│ Frontend (Next)  │ Healthy  │ 3000  │ 23 documents visible  │
│ PostgreSQL       │ Healthy  │ 5432  │ AGE 1.6.0 enabled     │
│ Ollama           │ Running  │ 11434 │ gemma3:12b loaded     │
└───────────────────────────────────────────────────────────────┘
```

### 2. Document Visibility

**Initial Concern**: Conversation summary indicated "Documents (0)" displayed despite successful pipeline.

**Actual Finding**: This was a **transient loading state**, NOT a bug.

**Evidence**:

- Page title showed "Documents (23)" immediately
- Snapshot after load showed 23 documents with full details
- API endpoint returns 23 documents correctly

```bash
curl "http://localhost:8080/api/v1/documents" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000003"

# Returns: {"documents":[...23 items...],"total":23,...}
```

### 3. Document Storage Architecture

**Key Finding**: Documents metadata stored in **KV storage**, not SQL `documents` table.

```
┌─────────────────────────────────────────────────────────────┐
│                    Storage Architecture                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  PDF Upload                                                 │
│      │                                                      │
│      ▼                                                      │
│  ┌────────────────┐     ┌────────────────┐                  │
│  │ pdf_documents  │────►│ PostgreSQL     │                  │
│  │ (SQL table)    │     │ Binary storage │                  │
│  └────────────────┘     └────────────────┘                  │
│      │                                                      │
│      │ Task Processing                                      │
│      ▼                                                      │
│  ┌────────────────┐     ┌────────────────┐                  │
│  │ tasks          │────►│ PostgreSQL     │ (OODA-06 fix)    │
│  │ (SQL table)    │     │ Persistent     │                  │
│  └────────────────┘     └────────────────┘                  │
│      │                                                      │
│      │ Metadata Updates                                     │
│      ▼                                                      │
│  ┌────────────────┐     ┌────────────────┐                  │
│  │ {doc_id}-      │────►│ KV Storage     │ (Memory/PG)      │
│  │ metadata       │     │ Fast Access    │                  │
│  └────────────────┘     └────────────────┘                  │
│      │                                                      │
│      │ GET /documents                                       │
│      ▼                                                      │
│  ┌────────────────┐                                         │
│  │ list_documents │ Reads from KV storage                   │
│  │ handler        │ NOT from SQL documents table            │
│  └────────────────┘                                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4. Task Persistence Verification

```sql
SELECT track_id, status, retry_count FROM tasks
WHERE workspace_id = '00000000-0000-0000-0000-000000000003';

-- Result:
-- pdf-21f40259-0051-4616-adf9-d23235e57d52 | indexed | 1
```

Tasks are persisted correctly in PostgreSQL as fixed in OODA-06.

### 5. Side-by-Side Viewer Verification

**Tested Document**: AI_Services\_\_Elitizon.pdf

**Results**:

- ✅ PDF viewer renders correctly (left panel)
- ✅ Markdown rendered with proper formatting (right panel)
- ✅ Page navigation works (1/5 pages)
- ✅ Zoom controls functional
- ✅ Headings, bold, lists, paragraphs all rendered

### 6. Database Statistics

```sql
-- Graph storage
SELECT COUNT(*) FROM "eq_eq_default_graph"."Node";    -- 2801 nodes
SELECT COUNT(*) FROM "eq_eq_default_graph"."EDGE";    -- 2219 edges

-- Vector storage
SELECT COUNT(*) FROM eq_eq_default_ws_00000000_vectors; -- 149 vectors

-- Documents by status
-- completed: 15
-- failed: 6
-- processing: 1
-- cancelled: 1
```

## Key Observations

### What's Working

1. **PDF to Markdown extraction** - Fully functional
2. **Entity extraction** - Working (with timeout handling)
3. **Relationship extraction** - Working
4. **Embedding generation** - Working
5. **Task persistence** - Fixed in OODA-06, verified working
6. **Frontend document list** - Shows all 23 documents
7. **Side-by-side viewer** - PDF + Markdown displayed correctly

### Non-Issues Identified

1. **"Documents (0)" display** - Was a loading state, not a bug
2. **SQL documents table empty** - By design, documents use KV storage
3. **No critical bugs found** - System operating correctly

### Minor Issues Observed

1. **lighrag_2410.05779v3.pdf stuck at "Converting PDF"** - Old task from before OODA-06 fix
2. **6 failed documents** - Due to Ollama timeouts (expected behavior)

## Evidence Collected

### API Response Sample

```json
{
  "documents": [
    {
      "id": "22b86f1b-093a-4d56-aa23-094752389775",
      "title": "AI_Services__Elitizon.pdf",
      "status": "completed",
      "entity_count": 20,
      "source_type": "pdf",
      "pdf_id": "ec2174c0-b6e2-4ed4-842f-a80f835c3f5a"
    }
    // ... 22 more documents
  ],
  "total": 23,
  "status_counts": {
    "completed": 15,
    "failed": 6,
    "processing": 1,
    "cancelled": 1
  }
}
```

### Browser Snapshot Evidence

- Page title: "Documents (23) - EdgeQuake"
- Table headers: Select, Title, Status, Entities, Cost, Created
- First row: AI_Services\_\_Elitizon.pdf, Completed, 20 entities, $0.00063, 30 min ago

## Conclusion

The system is operating correctly. The initial concern about document visibility was due to:

1. Misinterpretation of React loading state
2. Confusion about KV storage vs SQL table design

No code changes required for this iteration - system verification complete.
