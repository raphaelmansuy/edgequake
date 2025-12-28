# API Contracts: Ingestion Pipeline

> Document ID: API-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Overview](#1-overview)
2. [Ingestion Endpoints](#2-ingestion-endpoints)
3. [Progress & Status Endpoints](#3-progress--status-endpoints)
4. [Lineage Endpoints](#4-lineage-endpoints)
5. [Cost Management Endpoints](#5-cost-management-endpoints)
6. [Document Management Endpoints](#6-document-management-endpoints)
7. [WebSocket Events](#7-websocket-events)

---

## 1. Overview

### 1.1 Base URL

```
https://api.edgequake.io/api/v1
```

### 1.2 Authentication

All endpoints require Bearer token authentication:
```
Authorization: Bearer <access_token>
```

### 1.3 Common Headers

```
Content-Type: application/json
X-Tenant-ID: <tenant_id>
X-Workspace-ID: <workspace_id>
```

### 1.4 Error Response Format

```json
{
  "error": {
    "code": "E001",
    "message": "Human readable message",
    "details": {
      "field": "additional context"
    }
  }
}
```

---

## 2. Ingestion Endpoints

### 2.1 Upload Document (Text)

Upload a document for ingestion via text content.

**Endpoint:** `POST /documents`

**Request:**
```json
{
  "content": "Document text content...",
  "filename": "document.txt",
  "metadata": {
    "source": "manual_upload",
    "author": "John Doe"
  },
  "config": {
    "chunk_size": 1200,
    "chunk_overlap": 100,
    "extraction_model": "gpt-4o-mini",
    "entity_types": ["PERSON", "ORGANIZATION", "CONCEPT"],
    "max_gleaning": 1,
    "enable_mapreduce_summary": true
  }
}
```

**Response (202 Accepted):**
```json
{
  "document_id": "doc_abc123",
  "track_id": "track_xyz789",
  "status": "pending",
  "message": "Document queued for processing",
  "created_at": "2024-12-28T10:00:00Z",
  "estimated_processing_time_seconds": 45,
  "links": {
    "status": "/api/v1/documents/track/track_xyz789",
    "document": "/api/v1/documents/doc_abc123",
    "lineage": "/api/v1/documents/doc_abc123/lineage"
  }
}
```

### 2.2 Upload Document (File)

Upload a document via multipart form.

**Endpoint:** `POST /documents/upload`

**Request:**
```
Content-Type: multipart/form-data

file: <binary data>
config: {"chunk_size": 1200, ...}
metadata: {"source": "file_upload"}
```

**Response (202 Accepted):**
```json
{
  "document_id": "doc_abc123",
  "track_id": "track_xyz789",
  "filename": "report.pdf",
  "size_bytes": 1024000,
  "mime_type": "application/pdf",
  "status": "pending",
  "created_at": "2024-12-28T10:00:00Z"
}
```

### 2.3 Upload Documents (Batch)

Upload multiple documents at once.

**Endpoint:** `POST /documents/upload/batch`

**Request:**
```
Content-Type: multipart/form-data

files[]: <binary data 1>
files[]: <binary data 2>
config: {"chunk_size": 1200, ...}
```

**Response (202 Accepted):**
```json
{
  "batch_id": "batch_def456",
  "documents": [
    {
      "document_id": "doc_abc123",
      "track_id": "track_xyz789",
      "filename": "report1.pdf",
      "status": "pending"
    },
    {
      "document_id": "doc_def456",
      "track_id": "track_uvw321",
      "filename": "report2.pdf",
      "status": "pending"
    }
  ],
  "total_documents": 2,
  "created_at": "2024-12-28T10:00:00Z"
}
```

### 2.4 Re-ingest Document

Re-process an existing document with new configuration.

**Endpoint:** `POST /documents/{document_id}/reingest`

**Request:**
```json
{
  "config": {
    "chunk_size": 800,
    "max_gleaning": 2
  },
  "clear_existing": true
}
```

**Response (202 Accepted):**
```json
{
  "document_id": "doc_abc123",
  "track_id": "track_new123",
  "previous_track_id": "track_xyz789",
  "status": "pending",
  "message": "Document queued for re-ingestion"
}
```

---

## 3. Progress & Status Endpoints

### 3.1 Get Ingestion Status

Get detailed status of an ingestion job.

**Endpoint:** `GET /documents/track/{track_id}`

**Response (200 OK):**
```json
{
  "track_id": "track_xyz789",
  "document_id": "doc_abc123",
  "status": "running",
  "progress": {
    "current_stage": "extracting",
    "completion_percentage": 45.5,
    "eta_seconds": 30,
    "latest_message": "Extracting entities from chunk 5/10",
    "stages": [
      {
        "stage": "preprocessing",
        "status": "completed",
        "total_items": 1,
        "completed_items": 1,
        "started_at": "2024-12-28T10:00:01Z",
        "completed_at": "2024-12-28T10:00:02Z"
      },
      {
        "stage": "chunking",
        "status": "completed",
        "total_items": 1,
        "completed_items": 1,
        "started_at": "2024-12-28T10:00:02Z",
        "completed_at": "2024-12-28T10:00:03Z"
      },
      {
        "stage": "extracting",
        "status": "running",
        "total_items": 10,
        "completed_items": 5,
        "started_at": "2024-12-28T10:00:03Z",
        "completed_at": null
      },
      {
        "stage": "merging",
        "status": "pending",
        "total_items": 0,
        "completed_items": 0,
        "started_at": null,
        "completed_at": null
      }
    ]
  },
  "started_at": "2024-12-28T10:00:00Z",
  "updated_at": "2024-12-28T10:00:15Z"
}
```

### 3.2 Get Ingestion Status (Completed)

Status response when ingestion is completed.

**Response (200 OK):**
```json
{
  "track_id": "track_xyz789",
  "document_id": "doc_abc123",
  "status": "completed",
  "progress": {
    "current_stage": "finalizing",
    "completion_percentage": 100.0,
    "latest_message": "Ingestion completed successfully"
  },
  "result": {
    "job_id": "track_xyz789",
    "document_id": "doc_abc123",
    "chunk_count": 10,
    "total_chunk_tokens": 12000,
    "avg_chunk_size": 1200,
    "entity_count": 25,
    "entities_created": 20,
    "entities_updated": 5,
    "unique_entity_types": ["PERSON", "ORGANIZATION", "CONCEPT"],
    "relationship_count": 15,
    "relationships_created": 12,
    "relationships_updated": 3,
    "unique_relationship_types": ["WORKS_AT", "KNOWS", "RELATED_TO"],
    "keywords": ["technology", "innovation", "leadership"],
    "processing_time_ms": 45000,
    "llm_calls": 12,
    "embedding_calls": 3,
    "extraction_model": "gpt-4o-mini",
    "embedding_model": "text-embedding-3-small"
  },
  "cost": {
    "total_cost_usd": 0.0045,
    "breakdown": {
      "extraction": {
        "api_calls": 10,
        "input_tokens": 15000,
        "output_tokens": 3000,
        "cost_usd": 0.0040
      },
      "gleaning": {
        "api_calls": 2,
        "input_tokens": 2000,
        "output_tokens": 500,
        "cost_usd": 0.0004
      },
      "embedding": {
        "api_calls": 3,
        "input_tokens": 5000,
        "output_tokens": 0,
        "cost_usd": 0.0001
      }
    }
  },
  "started_at": "2024-12-28T10:00:00Z",
  "completed_at": "2024-12-28T10:00:45Z"
}
```

### 3.3 List Active Ingestion Jobs

Get all active ingestion jobs for the workspace.

**Endpoint:** `GET /tasks`

**Query Parameters:**
- `status`: Filter by status (pending, running, completed, failed)
- `limit`: Maximum results (default: 50)
- `offset`: Pagination offset

**Response (200 OK):**
```json
{
  "tasks": [
    {
      "track_id": "track_xyz789",
      "document_id": "doc_abc123",
      "filename": "report.pdf",
      "status": "running",
      "progress_percentage": 45.5,
      "started_at": "2024-12-28T10:00:00Z"
    },
    {
      "track_id": "track_uvw321",
      "document_id": "doc_def456",
      "filename": "analysis.txt",
      "status": "pending",
      "progress_percentage": 0,
      "started_at": null
    }
  ],
  "total": 2,
  "limit": 50,
  "offset": 0
}
```

### 3.4 Cancel Ingestion Job

Cancel a running or pending ingestion job.

**Endpoint:** `POST /tasks/{track_id}/cancel`

**Response (200 OK):**
```json
{
  "track_id": "track_xyz789",
  "status": "cancelled",
  "message": "Ingestion job cancelled successfully",
  "cancelled_at": "2024-12-28T10:01:00Z"
}
```

### 3.5 Retry Failed Ingestion

Retry a failed ingestion job.

**Endpoint:** `POST /tasks/{track_id}/retry`

**Request (optional):**
```json
{
  "config_overrides": {
    "max_gleaning": 0
  }
}
```

**Response (202 Accepted):**
```json
{
  "track_id": "track_new456",
  "previous_track_id": "track_xyz789",
  "status": "pending",
  "message": "Ingestion job queued for retry"
}
```

---

## 4. Lineage Endpoints

### 4.1 Get Document Lineage

Get complete lineage for a document.

**Endpoint:** `GET /documents/{document_id}/lineage`

**Response (200 OK):**
```json
{
  "document_id": "doc_abc123",
  "document_name": "report.pdf",
  "job_id": "track_xyz789",
  "ingestion_config": {
    "chunk_size": 1200,
    "extraction_model": "gpt-4o-mini"
  },
  "summary": {
    "total_chunks": 10,
    "total_entities": 25,
    "total_relationships": 15
  },
  "chunks": [
    {
      "chunk_id": "doc_abc123-chunk-0",
      "chunk_index": 0,
      "start_line": 1,
      "end_line": 50,
      "token_count": 1200,
      "entities": ["JOHN_DOE", "ACME_CORP"],
      "relationships": ["JOHN_DOE->ACME_CORP:WORKS_AT"]
    },
    {
      "chunk_id": "doc_abc123-chunk-1",
      "chunk_index": 1,
      "start_line": 45,
      "end_line": 95,
      "token_count": 1180,
      "entities": ["ACME_CORP", "PROJECT_ALPHA"],
      "relationships": ["ACME_CORP->PROJECT_ALPHA:DEVELOPS"]
    }
  ],
  "entities": [
    {
      "entity_id": "JOHN_DOE",
      "entity_name": "John Doe",
      "entity_type": "PERSON",
      "source_chunks": ["doc_abc123-chunk-0"],
      "first_seen_line": 5
    }
  ],
  "created_at": "2024-12-28T10:00:45Z"
}
```

### 4.2 Get Entity Lineage

Get lineage for a specific entity across all documents.

**Endpoint:** `GET /graph/entities/{entity_id}/lineage`

**Response (200 OK):**
```json
{
  "entity_id": "JOHN_DOE",
  "entity_name": "John Doe",
  "entity_type": "PERSON",
  "sources": [
    {
      "document_id": "doc_abc123",
      "document_name": "report.pdf",
      "chunks": [
        {
          "chunk_id": "doc_abc123-chunk-0",
          "start_line": 1,
          "end_line": 50,
          "source_text": "John Doe is the CEO of Acme Corp..."
        }
      ],
      "first_extracted_at": "2024-12-28T10:00:15Z"
    },
    {
      "document_id": "doc_def456",
      "document_name": "analysis.txt",
      "chunks": [
        {
          "chunk_id": "doc_def456-chunk-2",
          "start_line": 80,
          "end_line": 95,
          "source_text": "According to John Doe..."
        }
      ],
      "first_extracted_at": "2024-12-28T11:00:00Z"
    }
  ],
  "total_extraction_count": 3,
  "description_history": [
    {
      "description": "John Doe is the CEO of Acme Corp",
      "source": "extraction",
      "created_at": "2024-12-28T10:00:15Z"
    },
    {
      "description": "John Doe is the CEO and founder of Acme Corp, a technology company",
      "source": "merge",
      "created_at": "2024-12-28T11:00:00Z"
    }
  ]
}
```

### 4.3 Get Chunk Details

Get details for a specific chunk including entities and relationships.

**Endpoint:** `GET /chunks/{chunk_id}`

**Response (200 OK):**
```json
{
  "chunk_id": "doc_abc123-chunk-0",
  "document_id": "doc_abc123",
  "document_name": "report.pdf",
  "content": "John Doe is the CEO of Acme Corp...",
  "position": {
    "index": 0,
    "start_offset": 0,
    "end_offset": 2500,
    "start_line": 1,
    "end_line": 50
  },
  "token_count": 1200,
  "entities": [
    {
      "id": "JOHN_DOE",
      "name": "John Doe",
      "type": "PERSON",
      "description": "CEO of Acme Corp"
    },
    {
      "id": "ACME_CORP",
      "name": "Acme Corp",
      "type": "ORGANIZATION",
      "description": "Technology company"
    }
  ],
  "relationships": [
    {
      "source": "JOHN_DOE",
      "target": "ACME_CORP",
      "type": "WORKS_AT",
      "description": "John Doe works at Acme Corp as CEO"
    }
  ],
  "extraction_metadata": {
    "model": "gpt-4o-mini",
    "gleaning_iterations": 1,
    "extraction_time_ms": 2500,
    "input_tokens": 1500,
    "output_tokens": 400
  }
}
```

---

## 5. Cost Management Endpoints

### 5.1 Get Ingestion Cost

Get cost details for a specific ingestion job.

**Endpoint:** `GET /costs/{track_id}`

**Response (200 OK):**
```json
{
  "track_id": "track_xyz789",
  "document_id": "doc_abc123",
  "total_cost_usd": 0.0045,
  "breakdown": {
    "extraction": {
      "api_calls": 10,
      "input_tokens": 15000,
      "output_tokens": 3000,
      "cost_usd": 0.0040,
      "model": "gpt-4o-mini"
    },
    "gleaning": {
      "api_calls": 2,
      "input_tokens": 2000,
      "output_tokens": 500,
      "cost_usd": 0.0004,
      "model": "gpt-4o-mini"
    },
    "summarization": {
      "api_calls": 0,
      "input_tokens": 0,
      "output_tokens": 0,
      "cost_usd": 0.0,
      "model": "gpt-4o-mini"
    },
    "embedding": {
      "api_calls": 3,
      "input_tokens": 5000,
      "output_tokens": 0,
      "cost_usd": 0.0001,
      "model": "text-embedding-3-small"
    }
  },
  "token_usage": {
    "total_input_tokens": 17000,
    "total_output_tokens": 3500,
    "total_embedding_tokens": 5000,
    "total_tokens": 25500
  },
  "calculated_at": "2024-12-28T10:00:45Z"
}
```

### 5.2 Get Workspace Cost Summary

Get aggregated cost summary for a workspace.

**Endpoint:** `GET /costs/summary`

**Query Parameters:**
- `start_date`: Start of date range (ISO 8601)
- `end_date`: End of date range (ISO 8601)
- `group_by`: Grouping (day, week, month)

**Response (200 OK):**
```json
{
  "workspace_id": "ws_123",
  "period": {
    "start": "2024-12-01T00:00:00Z",
    "end": "2024-12-28T23:59:59Z"
  },
  "summary": {
    "total_cost_usd": 15.45,
    "total_documents": 150,
    "total_tokens": 2500000,
    "average_cost_per_document": 0.103
  },
  "breakdown_by_operation": {
    "extraction": 12.50,
    "gleaning": 1.80,
    "summarization": 0.90,
    "embedding": 0.25
  },
  "breakdown_by_period": [
    {
      "period": "2024-12-01",
      "cost_usd": 5.20,
      "documents": 50
    },
    {
      "period": "2024-12-08",
      "cost_usd": 4.80,
      "documents": 45
    }
  ]
}
```

---

## 6. Document Management Endpoints

### 6.1 Suppress Document

Mark a document as deleted and clean up its graph contributions.

**Endpoint:** `DELETE /documents/{document_id}`

**Query Parameters:**
- `cascade`: Whether to remove orphaned entities (default: true)
- `hard_delete`: Whether to permanently delete (default: false)

**Response (200 OK):**
```json
{
  "document_id": "doc_abc123",
  "status": "deleted",
  "cascade_result": {
    "chunks_removed": 10,
    "entities_orphaned": 5,
    "entities_removed": 3,
    "relationships_removed": 8,
    "entities_updated": 2
  },
  "deleted_at": "2024-12-28T12:00:00Z",
  "message": "Document suppressed successfully"
}
```

### 6.2 Get Document Impact Analysis

Preview the impact of deleting a document before actually deleting.

**Endpoint:** `GET /documents/{document_id}/impact`

**Response (200 OK):**
```json
{
  "document_id": "doc_abc123",
  "document_name": "report.pdf",
  "impact": {
    "chunks_to_remove": 10,
    "entities_affected": [
      {
        "entity_id": "JOHN_DOE",
        "other_source_count": 2,
        "action": "update"
      },
      {
        "entity_id": "PROJECT_ALPHA",
        "other_source_count": 0,
        "action": "remove"
      }
    ],
    "relationships_affected": [
      {
        "relationship_id": "JOHN_DOE->ACME_CORP",
        "other_source_count": 1,
        "action": "update"
      }
    ],
    "total_entities_to_update": 3,
    "total_entities_to_remove": 2,
    "total_relationships_to_remove": 5
  }
}
```

### 6.3 Reprocess Failed Documents

Batch reprocess all failed documents.

**Endpoint:** `POST /documents/reprocess`

**Request:**
```json
{
  "max_documents": 100,
  "config_overrides": {
    "max_gleaning": 0
  }
}
```

**Response (202 Accepted):**
```json
{
  "batch_id": "batch_reprocess_123",
  "documents_queued": 15,
  "documents": [
    {
      "document_id": "doc_abc123",
      "track_id": "track_new123",
      "previous_error": "LLM rate limit exceeded"
    }
  ]
}
```

---

## 7. WebSocket Events

### 7.1 Connection

Connect to the WebSocket for real-time updates.

**Endpoint:** `wss://api.edgequake.io/ws`

**Authentication:**
```json
{
  "type": "auth",
  "token": "<access_token>"
}
```

### 7.2 Subscribe to Ingestion Updates

Subscribe to updates for a specific ingestion job.

**Client Message:**
```json
{
  "type": "subscribe",
  "channel": "ingestion",
  "track_id": "track_xyz789"
}
```

### 7.3 Progress Update Event

Server pushes progress updates.

**Server Message:**
```json
{
  "type": "progress",
  "track_id": "track_xyz789",
  "stage": "extracting",
  "completion_percentage": 45.5,
  "message": "Extracting entities from chunk 5/10",
  "timestamp": "2024-12-28T10:00:15Z"
}
```

### 7.4 Stage Completed Event

Server notifies when a stage completes.

**Server Message:**
```json
{
  "type": "stage_completed",
  "track_id": "track_xyz789",
  "stage": "extracting",
  "result": {
    "entities_extracted": 25,
    "relationships_extracted": 15
  },
  "next_stage": "merging",
  "timestamp": "2024-12-28T10:00:30Z"
}
```

### 7.5 Job Completed Event

Server notifies when job completes.

**Server Message:**
```json
{
  "type": "completed",
  "track_id": "track_xyz789",
  "document_id": "doc_abc123",
  "result": {
    "chunk_count": 10,
    "entity_count": 25,
    "relationship_count": 15,
    "processing_time_ms": 45000
  },
  "cost": {
    "total_cost_usd": 0.0045
  },
  "timestamp": "2024-12-28T10:00:45Z"
}
```

### 7.6 Error Event

Server notifies of errors.

**Server Message:**
```json
{
  "type": "error",
  "track_id": "track_xyz789",
  "error": {
    "code": "E003",
    "message": "LLM rate limit exceeded",
    "stage": "extracting",
    "recoverable": true
  },
  "timestamp": "2024-12-28T10:00:20Z"
}
```

---

## Appendix A: Error Codes

| Code | Description | HTTP Status | Recoverable |
|------|-------------|-------------|-------------|
| E001 | Invalid request format | 400 | No |
| E002 | Authentication failed | 401 | No |
| E003 | Rate limit exceeded | 429 | Yes |
| E004 | Document not found | 404 | No |
| E005 | Ingestion job not found | 404 | No |
| E006 | Document too large | 413 | No |
| E007 | LLM provider error | 502 | Yes |
| E008 | Storage error | 500 | Yes |
| E009 | Extraction failed | 500 | Yes |
| E010 | Embedding failed | 500 | Yes |

## Appendix B: Rate Limits

| Endpoint | Rate Limit | Window |
|----------|------------|--------|
| POST /documents | 100/hour | 1 hour |
| POST /documents/upload | 50/hour | 1 hour |
| GET /documents/track/* | 1000/hour | 1 hour |
| WebSocket connections | 10/user | Concurrent |

---
