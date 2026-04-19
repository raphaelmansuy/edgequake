---
title: 'Extended API Reference'
---

# Extended API Reference

> **Additional Endpoints for Tasks, Pipeline, Costs, and Lineage**

This document covers advanced API endpoints not included in the main [REST API Reference](/docs/api-reference/rest-api/).

---

## Table of Contents

- [Ollama Emulation API](#ollama-emulation-api)
- [Tasks API](#tasks-api)
- [Pipeline API](#pipeline-api)
- [Cost Tracking API](#cost-tracking-api)
- [Lineage API](#lineage-api)
- [Tenants API](#tenants-api)
- [Advanced Document Endpoints](#advanced-document-endpoints)

---

## Ollama Emulation API

EdgeQuake emulates the Ollama API, enabling compatibility with tools like OpenWebUI.

### Base URL: `/api` (not `/api/v1`)

### GET /api/version

Get Ollama-compatible version.

```bash
curl http://localhost:8080/api/version
```

**Response**:

```json
{
  "version": "0.10.x"
}
```

### GET /api/tags

List available models (Ollama format).

```bash
curl http://localhost:8080/api/tags
```

**Response**:

```json
{
  "models": [
    {
      "name": "gemma3:12b",
      "model": "gemma3:12b",
      "modified_at": "2024-01-15T10:30:00Z",
      "size": 12000000000,
      "digest": "sha256:...",
      "details": {
        "format": "gguf",
        "family": "gemma",
        "parameter_size": "12B",
        "quantization_level": "Q4_K_M"
      }
    }
  ]
}
```

### GET /api/ps

List running model processes.

```bash
curl http://localhost:8080/api/ps
```

**Response**:

```json
{
  "models": [
    {
      "name": "gemma3:12b",
      "model": "gemma3:12b",
      "size": 7200000000,
      "digest": "sha256:...",
      "expires_at": "2024-01-15T11:30:00Z"
    }
  ]
}
```

### POST /api/generate

Generate text completion (Ollama format).

```bash
curl -X POST http://localhost:8080/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma3:12b",
    "prompt": "Why is the sky blue?",
    "stream": false
  }'
```

**Response** (non-streaming):

```json
{
  "model": "gemma3:12b",
  "created_at": "2024-01-15T10:30:00Z",
  "response": "The sky appears blue because...",
  "done": true,
  "context": [1, 2, 3],
  "total_duration": 1200000000,
  "load_duration": 100000000,
  "prompt_eval_count": 10,
  "prompt_eval_duration": 50000000,
  "eval_count": 100,
  "eval_duration": 1000000000
}
```

### POST /api/chat

Chat completion (Ollama format).

```bash
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma3:12b",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

**Response**:

```json
{
  "model": "gemma3:12b",
  "created_at": "2024-01-15T10:30:00Z",
  "message": {
    "role": "assistant",
    "content": "Hello! How can I help you today?"
  },
  "done": true,
  "total_duration": 800000000,
  "eval_count": 15
}
```

---

## Tasks API

Background task management for long-running operations.

### GET /api/v1/tasks

List all tasks.

**Query Parameters**:

| Parameter | Type    | Default | Description       |
| --------- | ------- | ------- | ----------------- |
| `status`  | string  | all     | Filter by status  |
| `limit`   | integer | 50      | Max results       |
| `offset`  | integer | 0       | Pagination offset |

**Task Status Values**:

- `pending` - Waiting to start
- `running` - Currently executing
- `completed` - Successfully finished
- `failed` - Failed with error
- `cancelled` - User cancelled

```bash
curl http://localhost:8080/api/v1/tasks?status=running \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "tasks": [
    {
      "id": "task-uuid",
      "type": "document_processing",
      "status": "running",
      "progress": 65,
      "document_id": "doc-uuid",
      "started_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:31:00Z"
    }
  ],
  "total": 1
}
```

### GET /api/v1/tasks/:id

Get task details.

```bash
curl http://localhost:8080/api/v1/tasks/task-uuid
```

**Response**:

```json
{
  "id": "task-uuid",
  "type": "document_processing",
  "status": "running",
  "progress": 65,
  "document_id": "doc-uuid",
  "stages": {
    "chunking": "completed",
    "extraction": "running",
    "merging": "pending",
    "embedding": "pending"
  },
  "stats": {
    "chunks_processed": 13,
    "chunks_total": 20,
    "entities_found": 45,
    "relationships_found": 32
  },
  "started_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:31:00Z"
}
```

### POST /api/v1/tasks/:id/cancel

Cancel a running task.

```bash
curl -X POST http://localhost:8080/api/v1/tasks/task-uuid/cancel
```

**Response**:

```json
{
  "id": "task-uuid",
  "status": "cancelled",
  "message": "Task cancellation requested"
}
```

### POST /api/v1/tasks/:id/retry

Retry a failed task.

```bash
curl -X POST http://localhost:8080/api/v1/tasks/task-uuid/retry
```

**Response**:

```json
{
  "id": "new-task-uuid",
  "status": "pending",
  "message": "Task queued for retry"
}
```

### GET /api/v1/documents/track/:track_id

Track document processing status (alias for task status).

```bash
curl http://localhost:8080/api/v1/documents/track/task-uuid
```

---

## Pipeline API

Pipeline management and queue monitoring.

### GET /api/v1/pipeline/status

Get current pipeline status.

```bash
curl http://localhost:8080/api/v1/pipeline/status \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "status": "running",
  "active_tasks": 3,
  "queue_depth": 12,
  "workers": {
    "total": 4,
    "busy": 3,
    "idle": 1
  },
  "rates": {
    "documents_per_minute": 2.5,
    "chunks_per_minute": 45,
    "embeddings_per_minute": 120
  }
}
```

### POST /api/v1/pipeline/cancel

Cancel all pending tasks in the workspace.

```bash
curl -X POST http://localhost:8080/api/v1/pipeline/cancel \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "cancelled": 5,
  "message": "5 tasks cancelled"
}
```

### GET /api/v1/pipeline/queue-metrics

Get queue metrics (for monitoring dashboards).

```bash
curl http://localhost:8080/api/v1/pipeline/queue-metrics \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "queue": {
    "pending": 10,
    "running": 3,
    "failed": 1
  },
  "throughput": {
    "last_minute": 5,
    "last_hour": 120,
    "last_day": 2500
  },
  "latency": {
    "p50_ms": 500,
    "p95_ms": 2000,
    "p99_ms": 5000
  }
}
```

---

## Cost Tracking API

Track LLM usage and costs.

### GET /api/v1/pipeline/costs/pricing

Get current model pricing.

```bash
curl http://localhost:8080/api/v1/pipeline/costs/pricing
```

**Response**:

```json
{
  "models": [
    {
      "id": "gpt-5-nano",
      "provider": "openai",
      "input_cost_per_1k_tokens": 0.00015,
      "output_cost_per_1k_tokens": 0.0006
    },
    {
      "id": "text-embedding-3-small",
      "provider": "openai",
      "input_cost_per_1k_tokens": 0.00002
    },
    {
      "id": "gemma3:12b",
      "provider": "ollama",
      "input_cost_per_1k_tokens": 0,
      "output_cost_per_1k_tokens": 0
    }
  ]
}
```

### POST /api/v1/pipeline/costs/estimate

Estimate processing cost for a document.

```bash
curl -X POST http://localhost:8080/api/v1/pipeline/costs/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "content_length": 50000,
    "llm_model": "gpt-5-nano",
    "embedding_model": "text-embedding-3-small"
  }'
```

**Response**:

```json
{
  "estimated_chunks": 50,
  "estimated_tokens": {
    "extraction": 25000,
    "embedding": 15000,
    "query": 2000
  },
  "estimated_cost_usd": {
    "extraction": 0.0185,
    "embedding": 0.0003,
    "total": 0.0188
  }
}
```

### GET /api/v1/costs/summary

Get cost summary for workspace.

```bash
curl http://localhost:8080/api/v1/costs/summary \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "period": "current_month",
  "total_cost_usd": 12.45,
  "breakdown": {
    "extraction": 8.5,
    "embedding": 1.25,
    "queries": 2.7
  },
  "usage": {
    "documents_processed": 125,
    "queries_executed": 450,
    "tokens_used": 2500000
  }
}
```

### GET /api/v1/costs/history

Get cost history.

**Query Parameters**:

| Parameter     | Type   | Default | Description                   |
| ------------- | ------ | ------- | ----------------------------- |
| `start_date`  | string | 30d ago | Start date (ISO 8601)         |
| `end_date`    | string | now     | End date (ISO 8601)           |
| `granularity` | string | day     | Aggregation (hour, day, week) |

```bash
curl "http://localhost:8080/api/v1/costs/history?granularity=day" \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/costs/budget

Get budget status.

```bash
curl http://localhost:8080/api/v1/costs/budget \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "budget_usd": 100.0,
  "spent_usd": 45.5,
  "remaining_usd": 54.5,
  "percent_used": 45.5,
  "period": "monthly",
  "alert_threshold": 80,
  "projected_end_of_period": 95.2
}
```

### PATCH /api/v1/costs/budget

Update budget settings.

```bash
curl -X PATCH http://localhost:8080/api/v1/costs/budget \
  -H "Content-Type: application/json" \
  -d '{
    "budget_usd": 150.00,
    "alert_threshold": 75
  }'
```

---

## Lineage API

Track data provenance through the pipeline.

### GET /api/v1/lineage/entities/:entity_name

Get entity lineage showing origin documents and chunks.

```bash
curl http://localhost:8080/api/v1/lineage/entities/ENTITY_NAME \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "entity": {
    "id": "ENTITY_NAME",
    "type": "PERSON",
    "description": "A key figure in..."
  },
  "sources": [
    {
      "document_id": "doc-uuid-1",
      "document_title": "Document 1",
      "chunk_id": "chunk-uuid-1",
      "chunk_index": 5,
      "extraction_date": "2024-01-15T10:30:00Z",
      "confidence": 0.92
    },
    {
      "document_id": "doc-uuid-2",
      "document_title": "Document 2",
      "chunk_id": "chunk-uuid-2",
      "chunk_index": 12,
      "extraction_date": "2024-01-15T11:00:00Z",
      "confidence": 0.88
    }
  ],
  "merge_history": [
    {
      "date": "2024-01-15T11:00:00Z",
      "merged_from": "ENTITY_NAME_VARIANT",
      "reason": "Case-insensitive match"
    }
  ]
}
```

### GET /api/v1/lineage/documents/:document_id

Get document lineage showing extracted entities and relationships.

```bash
curl http://localhost:8080/api/v1/lineage/documents/doc-uuid \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "document": {
    "id": "doc-uuid",
    "title": "Document Title",
    "status": "completed"
  },
  "chunks": [
    {
      "id": "chunk-uuid-1",
      "index": 0,
      "entities_extracted": 5,
      "relationships_extracted": 3
    }
  ],
  "entities_contributed": [
    {
      "id": "ENTITY_NAME",
      "type": "PERSON",
      "is_primary_source": true
    }
  ],
  "relationships_contributed": [
    {
      "source": "ENTITY_A",
      "target": "ENTITY_B",
      "type": "WORKS_WITH"
    }
  ]
}
```

---

## Tenants API

Multi-tenant management.

### POST /api/v1/tenants

Create a new tenant.

```bash
curl -X POST http://localhost:8080/api/v1/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Acme Corp",
    "slug": "acme",
    "plan": "enterprise"
  }'
```

### GET /api/v1/tenants

List all tenants.

### GET /api/v1/tenants/:tenant_id

Get tenant details.

### PUT /api/v1/tenants/:tenant_id

Update tenant.

### DELETE /api/v1/tenants/:tenant_id

Delete tenant and all data.

### POST /api/v1/tenants/:tenant_id/workspaces

Create workspace within tenant.

```bash
curl -X POST http://localhost:8080/api/v1/tenants/tenant-uuid/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Project",
    "slug": "research",
    "llm_provider": "openai",
    "llm_model": "gpt-5-nano"
  }'
```

### GET /api/v1/tenants/:tenant_id/workspaces

List workspaces in tenant.

---

## Advanced Document Endpoints

### POST /api/v1/documents/upload

File upload via multipart form.

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: workspace-uuid" \
  -F "file=@document.pdf" \
  -F "title=My Document" \
  -F "metadata={\"category\":\"research\"}"
```

### POST /api/v1/documents/upload/batch

Batch file upload.

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload/batch \
  -H "X-Workspace-ID: workspace-uuid" \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.pdf" \
  -F "files=@doc3.pdf"
```

### POST /api/v1/documents/scan

Scan a directory for documents.

```bash
curl -X POST http://localhost:8080/api/v1/documents/scan \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/documents",
    "recursive": true,
    "extensions": [".pdf", ".txt", ".md"]
  }'
```

### POST /api/v1/documents/reprocess

Reprocess all failed documents.

```bash
curl -X POST http://localhost:8080/api/v1/documents/reprocess \
  -H "X-Workspace-ID: workspace-uuid"
```

### POST /api/v1/documents/recover-stuck

Recover documents stuck in processing state.

```bash
curl -X POST http://localhost:8080/api/v1/documents/recover-stuck \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/documents/:id/deletion-impact

Analyze impact of deleting a document.

```bash
curl http://localhost:8080/api/v1/documents/doc-uuid/deletion-impact
```

**Response**:

```json
{
  "document_id": "doc-uuid",
  "entities_affected": 15,
  "entities_to_delete": 5,
  "entities_to_update": 10,
  "relationships_affected": 25,
  "relationships_to_delete": 12,
  "relationships_to_update": 13
}
```

### POST /api/v1/documents/:id/retry-chunks

Retry failed chunks for a document.

```bash
curl -X POST http://localhost:8080/api/v1/documents/doc-uuid/retry-chunks
```

### GET /api/v1/documents/:id/failed-chunks

List failed chunks for a document.

```bash
curl http://localhost:8080/api/v1/documents/doc-uuid/failed-chunks
```

---

## Workspace Advanced Endpoints

### GET /api/v1/workspaces/:id/stats

Get detailed workspace statistics.

```bash
curl http://localhost:8080/api/v1/workspaces/workspace-uuid/stats
```

**Response**:

```json
{
  "workspace_id": "workspace-uuid",
  "documents": {
    "total": 150,
    "completed": 145,
    "processing": 3,
    "failed": 2
  },
  "chunks": {
    "total": 3500,
    "avg_per_document": 23
  },
  "entities": {
    "total": 1200,
    "by_type": {
      "PERSON": 250,
      "ORGANIZATION": 180,
      "CONCEPT": 770
    }
  },
  "relationships": {
    "total": 3200
  },
  "storage": {
    "documents_bytes": 45000000,
    "embeddings_bytes": 120000000,
    "total_bytes": 165000000
  }
}
```

### GET /api/v1/workspaces/:id/metrics-history

Get historical metrics.

```bash
curl "http://localhost:8080/api/v1/workspaces/workspace-uuid/metrics-history?days=7"
```

### POST /api/v1/workspaces/:id/metrics-snapshot

Trigger a metrics snapshot.

```bash
curl -X POST http://localhost:8080/api/v1/workspaces/workspace-uuid/metrics-snapshot
```

### POST /api/v1/workspaces/:id/rebuild-embeddings

Rebuild all embeddings (e.g., after model change).

```bash
curl -X POST http://localhost:8080/api/v1/workspaces/workspace-uuid/rebuild-embeddings \
  -H "Content-Type: application/json" \
  -d '{
    "embedding_model": "text-embedding-3-large",
    "embedding_dimension": 3072
  }'
```

### POST /api/v1/workspaces/:id/rebuild-knowledge-graph

Rebuild knowledge graph (re-extract entities).

```bash
curl -X POST http://localhost:8080/api/v1/workspaces/workspace-uuid/rebuild-knowledge-graph \
  -H "Content-Type: application/json" \
  -d '{
    "llm_model": "gpt-4o"
  }'
```

### POST /api/v1/workspaces/:id/reprocess-documents

Reprocess all documents.

```bash
curl -X POST http://localhost:8080/api/v1/workspaces/workspace-uuid/reprocess-documents
```

---

## Models & Providers API

### GET /api/v1/models

List all configured models.

### GET /api/v1/models/llm

List LLM models only.

### GET /api/v1/models/embedding

List embedding models only.

### GET /api/v1/models/health

Check provider health.

```bash
curl http://localhost:8080/api/v1/models/health
```

**Response**:

```json
{
  "providers": [
    {
      "name": "openai",
      "status": "healthy",
      "latency_ms": 125
    },
    {
      "name": "ollama",
      "status": "healthy",
      "latency_ms": 15
    }
  ]
}
```

### GET /api/v1/models/:provider

Get provider details.

### GET /api/v1/models/:provider/:model

Get specific model details.

### GET /api/v1/settings/providers

List available providers.

### GET /api/v1/settings/provider/status

Get current provider status.

---

## See Also

- [REST API Reference](/docs/api-reference/rest-api/) - Core endpoints
- [Configuration Reference](/docs/operations/configuration/) - Environment variables
- [Troubleshooting](/docs/troubleshooting/common-issues/) - Debugging API issues
