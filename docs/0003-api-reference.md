# EdgeQuake API Reference

> Complete REST API documentation for EdgeQuake Server

**Version**: 0.1.0 | **Base URL**: `http://localhost:8080`

> **Code Reference**: See [edgequake/crates/edgequake-api/src/routes.rs](../edgequake/crates/edgequake-api/src/routes.rs) for route definitions and [edgequake/crates/edgequake-api/src/handlers/](../edgequake/crates/edgequake-api/src/handlers/) for handler implementations

---

## Table of Contents

1. [Overview](#overview)
2. [Health Endpoints](#health-endpoints)
3. [Ollama Emulation API](#ollama-emulation-api)
4. [Authentication](#authentication)
5. [User Endpoints](#user-endpoints)
6. [API Key Endpoints](#api-key-endpoints)
7. [Tenant Endpoints](#tenant-endpoints)
8. [Workspace Endpoints](#workspace-endpoints)
9. [Document Endpoints](#document-endpoints)
10. [Query Endpoints](#query-endpoints)
11. [Graph Endpoints](#graph-endpoints)
12. [Entity Endpoints](#entity-endpoints)
13. [Relationship Endpoints](#relationship-endpoints)
14. [Task Endpoints](#task-endpoints)
15. [Pipeline Endpoints](#pipeline-endpoints)
16. [Error Handling](#error-handling)

---

## Overview

### Base URL Structure

```
http://localhost:8080
├── /health          # Health check (root level)
├── /ready           # Readiness check
├── /live            # Liveness check
├── /metrics         # Prometheus metrics
├── /api/            # Ollama-compatible API
│   ├── version      # Ollama version
│   ├── tags         # List models
│   ├── ps           # Running models
│   ├── generate     # Text generation
│   └── chat         # Chat completion
└── /api/v1/         # API version 1
    ├── auth/        # Authentication
    ├── users/       # User management
    ├── api-keys/    # API key management
    ├── tenants/     # Tenant management
    ├── workspaces/  # Workspace management
    ├── documents/   # Document management
    ├── query/       # Query execution
    ├── graph/       # Knowledge graph
    ├── tasks/       # Task management
    └── pipeline/    # Pipeline control
```

### Content Type

All API requests and responses use JSON:

```http
Content-Type: application/json
Accept: application/json
```

### Authentication

EdgeQuake supports JWT bearer token and API key authentication:

```http
Authorization: Bearer <JWT_TOKEN>
# or
X-API-Key: <API_KEY>
```

---

## Health Endpoints

### GET `/health`

Health check endpoint.

```http
GET /health
```

**Response**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "components": {
    "database": "connected",
    "llm_provider": "openai",
    "storage": "ready"
  }
}
```

### GET `/ready`

Kubernetes readiness probe.

```http
GET /ready
```

**Response**

```json
{
  "status": "ready"
}
```

### GET `/live`

Kubernetes liveness probe.

```http
GET /live
```

**Response**

```json
{
  "status": "alive"
}
```

### GET `/metrics`

Prometheus metrics endpoint.

```http
GET /metrics
```

---

## Ollama Emulation API

EdgeQuake provides Ollama-compatible API endpoints, allowing it to work as a drop-in replacement for Ollama with tools like OpenWebUI.

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/ollama.rs](../edgequake/crates/edgequake-api/src/handlers/ollama.rs)

### Query Mode Prefixes

Chat messages can include a prefix to select the query mode:

| Prefix | Mode | Description |
|--------|------|-------------|
| `/local` | Local | Entity-centric query |
| `/global` | Global | Relationship-centric query |
| `/naive` | Naive | Chunk-only vector search |
| `/hybrid` | Hybrid | Combined local + global (default) |
| `/mix` | Mix | All strategies combined |
| `/bypass` | Bypass | Direct LLM (no RAG) |
| `/context` | Context | Return context only |

**Example:**
```
/local Who is Marie Curie?
```

### GET `/api/version`

Get Ollama version information.

```http
GET /api/version
```

**Response**

```json
{
  "version": "0.9.3"
}
```

### GET `/api/tags`

List available models.

```http
GET /api/tags
```

**Response**

```json
{
  "models": [
    {
      "name": "edgequake:latest",
      "model": "edgequake:latest",
      "size": 7000000000,
      "digest": "sha256:edgequake-rag-v1",
      "modified_at": "2025-12-25T00:00:00Z",
      "details": {
        "format": "gguf",
        "family": "edgequake",
        "families": ["edgequake"],
        "parameter_size": "7B",
        "quantization_level": "Q4_0"
      }
    }
  ]
}
```

### GET `/api/ps`

List running models.

```http
GET /api/ps
```

**Response**

```json
{
  "models": [
    {
      "name": "edgequake:latest",
      "model": "edgequake:latest",
      "size": 7000000000,
      "digest": "sha256:edgequake-rag-v1",
      "expires_at": "2025-12-25T01:00:00Z"
    }
  ]
}
```

### POST `/api/generate`

Generate a text completion (Ollama-compatible).

```http
POST /api/generate
Content-Type: application/json
```

**Request Body**

```json
{
  "model": "edgequake",
  "prompt": "What did Marie Curie discover?",
  "stream": true
}
```

**Streaming Response (NDJSON)**

```json
{"model":"edgequake","created_at":"2025-12-25T14:30:00Z","response":"Marie","done":false}
{"model":"edgequake","created_at":"2025-12-25T14:30:00Z","response":" Curie","done":false}
{"model":"edgequake","created_at":"2025-12-25T14:30:01Z","response":" discovered","done":false}
{"model":"edgequake","created_at":"2025-12-25T14:30:01Z","response":" radium.","done":true,"total_duration":1500000000}
```

### POST `/api/chat`

Chat completion with conversation history (Ollama-compatible).

```http
POST /api/chat
Content-Type: application/json
```

**Request Body**

```json
{
  "model": "edgequake",
  "messages": [
    {"role": "user", "content": "Who discovered radium?"},
    {"role": "assistant", "content": "Marie Curie discovered radium."},
    {"role": "user", "content": "/local Tell me more about her."}
  ],
  "stream": true
}
```

**Response**

Same format as `/api/generate` with `message` field containing the assistant's response.

---

## Authentication

### POST `/api/v1/auth/login`

Authenticate and obtain JWT tokens.

```http
POST /api/v1/auth/login
Content-Type: application/json
```

**Request Body**

```json
{
  "username": "admin",
  "password": "password"
}
```

**Response**

```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "user-123",
    "username": "admin",
    "role": "admin"
  }
}
```

### POST `/api/v1/auth/refresh`

Refresh access token.

```http
POST /api/v1/auth/refresh
Content-Type: application/json
```

**Request Body**

```json
{
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

### POST `/api/v1/auth/logout`

Invalidate tokens.

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

### GET `/api/v1/auth/me`

Get current user information.

```http
GET /api/v1/auth/me
Authorization: Bearer <token>
```

---

## User Endpoints

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs)

### POST `/api/v1/users`

Create a new user.

```http
POST /api/v1/users
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "username": "newuser",
  "email": "newuser@example.com",
  "password": "securepassword",
  "role": "user"
}
```

| Field      | Type   | Required | Default | Description                              |
| ---------- | ------ | -------- | ------- | ---------------------------------------- |
| `username` | string | ✅       | -       | Unique username                          |
| `email`    | string | ✅       | -       | User email address                       |
| `password` | string | ✅       | -       | User password                            |
| `role`     | string | ❌       | `user`  | Role: `admin`, `user`, or `readonly`     |

**Response (201 Created)**

```json
{
  "user_id": "user-550e8400",
  "username": "newuser",
  "email": "newuser@example.com",
  "role": "user",
  "is_active": true,
  "created_at": "2025-12-25T14:30:00Z"
}
```

### GET `/api/v1/users`

List all users with pagination.

```http
GET /api/v1/users?offset=0&limit=20
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter | Type | Default | Description      |
| --------- | ---- | ------- | ---------------- |
| `offset`  | int  | 0       | Pagination offset|
| `limit`   | int  | 20      | Items per page   |

**Response**

```json
{
  "users": [
    {
      "user_id": "user-123",
      "username": "admin",
      "email": "admin@example.com",
      "role": "admin",
      "is_active": true,
      "created_at": "2025-12-24T10:00:00Z"
    }
  ],
  "total": 1,
  "offset": 0,
  "limit": 20
}
```

### GET `/api/v1/users/{user_id}`

Get user details by ID.

```http
GET /api/v1/users/user-123
Authorization: Bearer <token>
```

### DELETE `/api/v1/users/{user_id}`

Delete a user.

```http
DELETE /api/v1/users/user-123
Authorization: Bearer <token>
```

---

## API Key Endpoints

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/auth.rs](../edgequake/crates/edgequake-api/src/handlers/auth.rs)

### POST `/api/v1/api-keys`

Create a new API key.

```http
POST /api/v1/api-keys
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "name": "Production API Key",
  "expires_in_days": 365
}
```

**Response (201 Created)**

```json
{
  "key_id": "key-550e8400",
  "name": "Production API Key",
  "api_key": "eq_live_abc123...",
  "created_at": "2025-12-25T14:30:00Z",
  "expires_at": "2026-12-25T14:30:00Z"
}
```

> ⚠️ **Important**: The `api_key` value is only shown once upon creation. Store it securely.

### GET `/api/v1/api-keys`

List all API keys for the current user.

```http
GET /api/v1/api-keys
Authorization: Bearer <token>
```

**Response**

```json
{
  "api_keys": [
    {
      "key_id": "key-550e8400",
      "name": "Production API Key",
      "key_prefix": "eq_live_abc...",
      "created_at": "2025-12-25T14:30:00Z",
      "expires_at": "2026-12-25T14:30:00Z",
      "last_used_at": "2025-12-25T15:00:00Z"
    }
  ]
}
```

### DELETE `/api/v1/api-keys/{key_id}`

Revoke an API key.

```http
DELETE /api/v1/api-keys/key-550e8400
Authorization: Bearer <token>
```

---

## Tenant Endpoints

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

### POST `/api/v1/tenants`

Create a new tenant.

```http
POST /api/v1/tenants
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "name": "Acme Corporation",
  "slug": "acme-corp",
  "description": "Production tenant for Acme",
  "plan": "pro"
}
```

| Field         | Type   | Required | Default | Description                              |
| ------------- | ------ | -------- | ------- | ---------------------------------------- |
| `name`        | string | ✅       | -       | Tenant name                              |
| `slug`        | string | ❌       | auto    | URL-friendly identifier                  |
| `description` | string | ❌       | null    | Tenant description                       |
| `plan`        | string | ❌       | `free`  | Plan: `free`, `basic`, `pro`, `enterprise` |

**Response (201 Created)**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Acme Corporation",
  "slug": "acme-corp",
  "plan": "pro",
  "is_active": true,
  "max_workspaces": 10,
  "created_at": "2025-12-25T14:30:00Z",
  "updated_at": "2025-12-25T14:30:00Z"
}
```

### GET `/api/v1/tenants`

List all tenants.

```http
GET /api/v1/tenants?offset=0&limit=20
Authorization: Bearer <token>
```

**Response**

```json
{
  "items": [...],
  "total": 5,
  "offset": 0,
  "limit": 20
}
```

### GET `/api/v1/tenants/{tenant_id}`

Get tenant details.

```http
GET /api/v1/tenants/550e8400-e29b-41d4-a716-446655440000
Authorization: Bearer <token>
```

### PUT `/api/v1/tenants/{tenant_id}`

Update a tenant.

```http
PUT /api/v1/tenants/550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "name": "Acme Corp Updated",
  "description": "Updated description",
  "plan": "enterprise",
  "is_active": true
}
```

### DELETE `/api/v1/tenants/{tenant_id}`

Delete a tenant and all its workspaces.

```http
DELETE /api/v1/tenants/550e8400-e29b-41d4-a716-446655440000
Authorization: Bearer <token>
```

---

## Workspace Endpoints

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

### POST `/api/v1/tenants/{tenant_id}/workspaces`

Create a new workspace within a tenant.

```http
POST /api/v1/tenants/550e8400-e29b-41d4-a716-446655440000/workspaces
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "name": "Production Workspace",
  "slug": "production",
  "description": "Main production environment",
  "max_documents": 10000
}
```

| Field           | Type   | Required | Default | Description                |
| --------------- | ------ | -------- | ------- | -------------------------- |
| `name`          | string | ✅       | -       | Workspace name             |
| `slug`          | string | ❌       | auto    | URL-friendly identifier    |
| `description`   | string | ❌       | null    | Workspace description      |
| `max_documents` | int    | ❌       | null    | Maximum documents allowed  |

**Response (201 Created)**

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440001",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Production Workspace",
  "slug": "production",
  "description": "Main production environment",
  "is_active": true,
  "max_documents": 10000,
  "created_at": "2025-12-25T14:30:00Z",
  "updated_at": "2025-12-25T14:30:00Z"
}
```

### GET `/api/v1/tenants/{tenant_id}/workspaces`

List workspaces within a tenant.

```http
GET /api/v1/tenants/550e8400-e29b-41d4-a716-446655440000/workspaces?offset=0&limit=20
Authorization: Bearer <token>
```

### GET `/api/v1/workspaces/{workspace_id}`

Get workspace details.

```http
GET /api/v1/workspaces/660e8400-e29b-41d4-a716-446655440001
Authorization: Bearer <token>
```

### PUT `/api/v1/workspaces/{workspace_id}`

Update a workspace.

```http
PUT /api/v1/workspaces/660e8400-e29b-41d4-a716-446655440001
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "name": "Updated Workspace Name",
  "description": "Updated description",
  "is_active": true,
  "max_documents": 20000
}
```

### DELETE `/api/v1/workspaces/{workspace_id}`

Delete a workspace and all its data.

```http
DELETE /api/v1/workspaces/660e8400-e29b-41d4-a716-446655440001
Authorization: Bearer <token>
```

### GET `/api/v1/workspaces/{workspace_id}/stats`

Get workspace statistics.

```http
GET /api/v1/workspaces/660e8400-e29b-41d4-a716-446655440001/stats
Authorization: Bearer <token>
```

**Response**

```json
{
  "workspace_id": "660e8400-e29b-41d4-a716-446655440001",
  "document_count": 150,
  "chunk_count": 750,
  "entity_count": 500,
  "relationship_count": 1200,
  "storage_used_bytes": 52428800,
  "last_activity_at": "2025-12-25T15:00:00Z"
}
```

---

## Document Endpoints

### POST `/api/v1/documents`

Upload a text document for processing.

```http
POST /api/v1/documents
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "content": "Marie Curie was a physicist who discovered radium...",
  "title": "Marie Curie Biography",
  "metadata": {
    "author": "Wikipedia",
    "source": "https://en.wikipedia.org/wiki/Marie_Curie"
  },
  "async_processing": false,
  "track_id": "batch-2024-001"
}
```

| Field              | Type   | Required | Default | Description            |
| ------------------ | ------ | -------- | ------- | ---------------------- |
| `content`          | string | ✅       | -       | Document text content  |
| `title`            | string | ❌       | null    | Document title         |
| `metadata`         | object | ❌       | null    | Additional metadata    |
| `async_processing` | bool   | ❌       | false   | Process asynchronously |
| `track_id`         | string | ❌       | auto    | Batch tracking ID      |

**Response (201 Created)**

```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processed",
  "track_id": "batch-2024-001",
  "chunk_count": 5,
  "entity_count": 12,
  "relationship_count": 8
}
```

**Async Response**

```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "task_id": "task_20251224_143000_abc123",
  "track_id": "batch-2024-001"
}
```

### POST `/api/v1/documents/upload`

Upload a file for processing.

```http
POST /api/v1/documents/upload
Content-Type: multipart/form-data
Authorization: Bearer <token>
```

**Form Data**

| Field  | Type | Description                         |
| ------ | ---- | ----------------------------------- |
| `file` | file | File to upload (txt, md, pdf, docx) |

**Response**

```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "task_id": "task_20251224_143000_abc123",
  "track_id": "upload_20251224_143000_abc123"
}
```

### POST `/api/v1/documents/upload/batch`

Upload multiple files.

```http
POST /api/v1/documents/upload/batch
Content-Type: multipart/form-data
Authorization: Bearer <token>
```

**Form Data**

| Field   | Type   | Description              |
| ------- | ------ | ------------------------ |
| `files` | file[] | Multiple files to upload |

### GET `/api/v1/documents`

List all documents with pagination.

```http
GET /api/v1/documents?page=1&page_size=20&status=processed
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter    | Type   | Default    | Description                                    |
| ------------ | ------ | ---------- | ---------------------------------------------- |
| `page`       | int    | 1          | Page number                                    |
| `page_size`  | int    | 20         | Items per page (max 100)                       |
| `status`     | string | -          | [PLANNED] Filter: pending, processing, processed, failed |
| `sort_by`    | string | created_at | [PLANNED] Sort field                                     |
| `sort_order` | string | desc       | [PLANNED] asc or desc                                    |

**Response**

```json
{
  "documents": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Marie Curie Biography",
      "status": "processed",
      "content_summary": "Marie Curie was a physicist who discovered radium...",
      "content_length": 5432,
      "chunk_count": 5,
      "entity_count": 12,
      "track_id": "batch-2024-001",
      "created_at": "2025-12-24T14:30:00Z",
      "processed_at": "2025-12-24T14:30:15Z"
    }
  ],
  "total": 150,
  "page": 1,
  "page_size": 20,
  "status_counts": {
    "pending": 5,
    "processing": 2,
    "completed": 140,
    "failed": 3
  }
}
```

### GET `/api/v1/documents/{document_id}`

Get document details.

```http
GET /api/v1/documents/550e8400-e29b-41d4-a716-446655440000
Authorization: Bearer <token>
```

### DELETE `/api/v1/documents/{document_id}`

Delete a document and associated data.

```http
DELETE /api/v1/documents/550e8400-e29b-41d4-a716-446655440000
Authorization: Bearer <token>
```

**Response**

```json
{
  "status": "success",
  "message": "Document deleted successfully",
  "deleted_chunks": 5,
  "affected_entities": 12,
  "affected_relationships": 8
}
```

### GET `/api/v1/documents/track/{track_id}`

Get status of a batch upload.

```http
GET /api/v1/documents/track/batch-2024-001
Authorization: Bearer <token>
```

**Response**

```json
{
  "track_id": "batch-2024-001",
  "created_at": "2025-12-24T14:30:00Z",
  "documents": [...],
  "total_count": 10,
  "status_summary": {
    "pending": 2,
    "processing": 1,
    "completed": 6,
    "failed": 1
  },
  "is_complete": false,
  "latest_message": "Processing document 7 of 10..."
}
```

### POST `/api/v1/documents/scan`

Scan a directory for new documents and queue them for processing.

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/documents.rs](../edgequake/crates/edgequake-api/src/handlers/documents.rs) - `scan_directory`

```http
POST /api/v1/documents/scan
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "path": "/data/documents",
  "recursive": true,
  "extensions": ["txt", "md", "pdf"],
  "max_files": 100,
  "track_id": "scan-2024-001"
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | ✅ | - | Directory path to scan |
| `recursive` | bool | ❌ | false | Scan subdirectories |
| `extensions` | string[] | ❌ | all | File extensions to include |
| `max_files` | int | ❌ | 1000 | Maximum files to queue |
| `track_id` | string | ❌ | auto | Batch tracking ID |

**Response (200 OK)**

```json
{
  "track_id": "scan-2024-001",
  "files_found": 45,
  "files_queued": 42,
  "skipped_files": [
    {
      "path": "/data/documents/image.png",
      "reason": "Extension .png not in filter list"
    }
  ],
  "message": "Queued 42 files for processing"
}
```

### POST `/api/v1/documents/reprocess`

Reprocess all failed documents.

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/documents.rs](../edgequake/crates/edgequake-api/src/handlers/documents.rs) - `reprocess_failed`

```http
POST /api/v1/documents/reprocess
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "track_id": "batch-2024-001",
  "max_documents": 50
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `track_id` | string | ❌ | all | Filter by original track ID |
| `max_documents` | int | ❌ | 100 | Maximum documents to reprocess |

**Response (200 OK)**

```json
{
  "track_id": "reprocess_20251225_143000_abc123",
  "count": 5,
  "requeued_ids": [
    "doc-001",
    "doc-015",
    "doc-023",
    "doc-042",
    "doc-056"
  ],
  "message": "Requeued 5 failed documents for processing"
}
```

---

## Query Endpoints

### POST `/api/v1/query`

Execute a RAG query.

```http
POST /api/v1/query
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "query": "What did Marie Curie discover?",
  "mode": "hybrid",
  "context_only": false,
  "max_results": 10,
  "conversation_history": [
    { "role": "user", "content": "Tell me about scientists" },
    { "role": "assistant", "content": "Scientists are..." }
  ],
  "enable_rerank": true,
  "rerank_top_k": 5
}
```

| Field                  | Type   | Required | Default | Description                                           |
| ---------------------- | ------ | -------- | ------- | ----------------------------------------------------- |
| `query`                | string | ✅       | -       | Query text                                            |
| `mode`                 | string | ❌       | hybrid  | Query mode: naive, local, global, hybrid, mix, bypass |
| `context_only`         | bool   | ❌       | false   | Return only context without LLM answer                |
| `max_results`          | int    | ❌       | 20      | Maximum context items                                 |
| `conversation_history` | array  | ❌       | []      | Previous conversation for context                     |
| `enable_rerank`        | bool   | ❌       | true    | Enable reranking                                      |
| `rerank_top_k`         | int    | ❌       | 10      | Top K after reranking                                 |

**Response**

```json
{
  "answer": "Marie Curie discovered radium and polonium. She conducted groundbreaking research on radioactivity...",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "chunk",
      "id": "chunk-001",
      "score": 0.92,
      "rerank_score": 0.95,
      "snippet": "Marie Curie discovered radium in 1898..."
    },
    {
      "source_type": "entity",
      "id": "MARIE_CURIE",
      "score": 0.88,
      "snippet": "Polish-French physicist and chemist..."
    },
    {
      "source_type": "relationship",
      "id": "MARIE_CURIE->RADIUM",
      "score": 0.85,
      "snippet": "MARIE_CURIE discovered RADIUM"
    }
  ],
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 120,
    "generation_time_ms": 850,
    "total_time_ms": 1015,
    "sources_retrieved": 15,
    "rerank_time_ms": 25
  },
  "conversation_id": "conv-550e8400",
  "reranked": true
}
```

### POST `/api/v1/query/stream`

Stream a RAG query response (Server-Sent Events).

```http
POST /api/v1/query/stream
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "query": "Explain the theory of relativity",
  "mode": "hybrid"
}
```

**Response (SSE Stream)**

```
event: data
data: {"type": "thinking", "content": "Analyzing query..."}

event: data
data: {"type": "sources", "sources": [...]}

event: data
data: {"type": "content", "content": "The theory of "}

event: data
data: {"type": "content", "content": "relativity..."}

event: data
data: {"type": "done", "stats": {...}}
```

---

## Graph Endpoints

### GET `/api/v1/graph`

Get knowledge graph overview.

```http
GET /api/v1/graph?max_nodes=100&depth=2
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter    | Type   | Default | Description      |
| ------------ | ------ | ------- | ---------------- |
| `start_node` | string | -       | Starting node ID |
| `depth`      | int    | 2       | Traversal depth  |
| `max_nodes`  | int    | 100     | Maximum nodes    |

**Response**

```json
{
  "nodes": [
    {
      "id": "MARIE_CURIE",
      "label": "MARIE_CURIE",
      "node_type": "PERSON",
      "description": "Polish-French physicist and chemist",
      "degree": 15,
      "properties": {
        "born": "1867",
        "nationality": "Polish-French"
      }
    }
  ],
  "edges": [
    {
      "source": "MARIE_CURIE",
      "target": "RADIUM",
      "edge_type": "DISCOVERED",
      "weight": 1.0,
      "properties": {
        "year": "1898"
      }
    }
  ],
  "is_truncated": false,
  "total_nodes": 150,
  "total_edges": 230
}
```

### GET `/api/v1/graph/nodes/{node_id}`

Get specific node details.

```http
GET /api/v1/graph/nodes/MARIE_CURIE
Authorization: Bearer <token>
```

### GET `/api/v1/graph/labels/search`

Search node labels.

```http
GET /api/v1/graph/labels/search?q=curie&limit=10
Authorization: Bearer <token>
```

**Response**

```json
{
  "labels": [
    {
      "id": "MARIE_CURIE",
      "node_type": "PERSON",
      "score": 0.95
    },
    {
      "id": "PIERRE_CURIE",
      "node_type": "PERSON",
      "score": 0.75
    }
  ]
}
```

### GET `/api/v1/graph/labels/popular`

Get popular node labels by degree.

```http
GET /api/v1/graph/labels/popular?limit=10
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter | Type | Default | Description              |
| --------- | ---- | ------- | ------------------------ |
| `limit`   | int  | 10      | Maximum labels to return |

**Response**

```json
{
  "labels": [
    {
      "id": "MARIE_CURIE",
      "node_type": "PERSON",
      "degree": 15
    },
    {
      "id": "RADIUM",
      "node_type": "ELEMENT",
      "degree": 8
    }
  ]
}
```

---

## Entity Endpoints

### POST `/api/v1/graph/entities`

Create a new entity.

```http
POST /api/v1/graph/entities
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "entity_name": "Albert Einstein",
  "entity_type": "PERSON",
  "description": "German-born theoretical physicist",
  "source_id": "manual_entry",
  "metadata": {
    "born": "1879",
    "nationality": "German-American"
  }
}
```

**Response (201 Created)**

```json
{
  "status": "success",
  "message": "Entity created successfully",
  "entity": {
    "id": "ALBERT_EINSTEIN",
    "entity_name": "ALBERT_EINSTEIN",
    "entity_type": "PERSON",
    "description": "German-born theoretical physicist",
    "source_id": "manual_entry",
    "created_at": "2025-12-24T14:30:00Z",
    "updated_at": "2025-12-24T14:30:00Z",
    "degree": 0,
    "metadata": {...}
  }
}
```

### GET `/api/v1/graph/entities/exists`

Check if entity exists.

```http
GET /api/v1/graph/entities/exists?entity_name=MARIE_CURIE
Authorization: Bearer <token>
```

**Response**

```json
{
  "exists": true,
  "entity_id": "MARIE_CURIE",
  "entity_type": "PERSON",
  "degree": 15
}
```

### GET `/api/v1/graph/entities/{entity_name}`

Get entity details.

```http
GET /api/v1/graph/entities/MARIE_CURIE
Authorization: Bearer <token>
```

### PUT `/api/v1/graph/entities/{entity_name}`

Update an entity.

```http
PUT /api/v1/graph/entities/MARIE_CURIE
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "entity_type": "SCIENTIST",
  "description": "Updated description...",
  "metadata": {...}
}
```

### DELETE `/api/v1/graph/entities/{entity_name}`

Delete an entity and its relationships.

```http
DELETE /api/v1/graph/entities/MARIE_CURIE?cascade=true
Authorization: Bearer <token>
```

### POST `/api/v1/graph/entities/merge`

Merge two entities.

```http
POST /api/v1/graph/entities/merge
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "source_entity": "MARIE_SKLODOWSKA",
  "target_entity": "MARIE_CURIE",
  "merge_strategy": "prefer_target"
}
```

**Response**

```json
{
  "status": "success",
  "message": "Entities merged successfully",
  "merged_entity": {...},
  "merge_details": {
    "source_entity_id": "MARIE_SKLODOWSKA",
    "target_entity_id": "MARIE_CURIE",
    "relationships_merged": 5,
    "duplicate_relationships_removed": 2
  }
}
```

---

## Relationship Endpoints

### POST `/api/v1/graph/relationships`

Create a relationship.

```http
POST /api/v1/graph/relationships
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body**

```json
{
  "source_entity": "MARIE_CURIE",
  "target_entity": "RADIUM",
  "relationship_type": "DISCOVERED",
  "description": "Marie Curie discovered radium in 1898",
  "weight": 1.0,
  "source_id": "manual_entry"
}
```

### GET `/api/v1/graph/relationships/{relationship_id}`

Get relationship details.

### PUT `/api/v1/graph/relationships/{relationship_id}`

Update a relationship.

### DELETE `/api/v1/graph/relationships/{relationship_id}`

Delete a relationship.

---

## Task Endpoints

### GET `/api/v1/tasks`

List tasks with filtering.

```http
GET /api/v1/tasks?status=processing&page=1&page_size=20
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter   | Type   | Default | Description      |
| ----------- | ------ | ------- | ---------------- |
| `status`    | string | -       | Filter: pending, processing, indexed, failed, cancelled |
| `task_type` | string | -       | Filter: upload, insert, scan, reindex |
| `page`      | int    | 1       | Page number      |
| `page_size` | int    | 20      | Items per page   |

**Response**

```json
{
  "tasks": [
    {
      "track_id": "task_20251224_143000_abc123",
      "task_type": "insert",
      "status": "processing",
      "progress": 0.75,
      "message": "Extracting entities...",
      "created_at": "2025-12-24T14:30:00Z"
    }
  ],
  "statistics": {
    "pending": 5,
    "processing": 2,
    "indexed": 150,
    "failed": 3
  }
}
```

### GET `/api/v1/tasks/{track_id}`

Get task status.

```http
GET /api/v1/tasks/task_20251224_143000_abc123
Authorization: Bearer <token>
```

### POST `/api/v1/tasks/{track_id}/cancel`

Cancel a running task.

```http
POST /api/v1/tasks/task_20251224_143000_abc123/cancel
Authorization: Bearer <token>
```

### POST `/api/v1/tasks/{track_id}/retry`

Retry a failed task.

```http
POST /api/v1/tasks/task_20251224_143000_abc123/retry
Authorization: Bearer <token>
```

---

## Pipeline Endpoints

> **Code Reference**: [edgequake/crates/edgequake-api/src/handlers/pipeline.rs](../edgequake/crates/edgequake-api/src/handlers/pipeline.rs)

### GET `/api/v1/pipeline/status`

Get current pipeline processing status.

```http
GET /api/v1/pipeline/status
Authorization: Bearer <token>
```

**Response**

```json
{
  "is_busy": true,
  "job_name": "batch-upload-001",
  "job_start": "2025-12-25T14:30:00Z",
  "total_documents": 50,
  "processed_documents": 25,
  "current_batch": 3,
  "total_batches": 5,
  "latest_message": "Processing document 25 of 50...",
  "history_messages": [
    {
      "timestamp": "2025-12-25T14:30:00Z",
      "level": "info",
      "message": "Pipeline started"
    },
    {
      "timestamp": "2025-12-25T14:31:00Z",
      "level": "info",
      "message": "Batch 1 completed"
    }
  ],
  "cancellation_requested": false,
  "pending_tasks": 25,
  "processing_tasks": 2,
  "completed_tasks": 23,
  "failed_tasks": 0
}
```

### POST `/api/v1/pipeline/cancel`

Request cancellation of the current pipeline job.

```http
POST /api/v1/pipeline/cancel
Authorization: Bearer <token>
```

**Response**

```json
{
  "status": "cancellation_requested",
  "message": "Pipeline cancellation has been requested. Processing will stop after the current batch."
}
```

---

## Error Handling

### Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Query cannot be empty",
    "details": {
      "field": "query",
      "constraint": "min_length"
    }
  },
  "status": 400
}
```

### Error Codes

| HTTP Status | Code              | Description                 |
| ----------- | ----------------- | --------------------------- |
| 400         | BAD_REQUEST       | Invalid request format      |
| 400         | VALIDATION_ERROR  | Request validation failed   |
| 401         | AUTH_REQUIRED     | Authentication required     |
| 401         | INVALID_TOKEN     | Invalid or expired token    |
| 403         | FORBIDDEN         | Insufficient permissions    |
| 404         | NOT_FOUND         | Resource not found          |
| 413         | PAYLOAD_TOO_LARGE | Document exceeds size limit |
| 429         | RATE_LIMITED      | Too many requests           |
| 500         | INTERNAL_ERROR    | Server error                |

---

## Rate Limits

| Endpoint Type   | Limit               |
| --------------- | ------------------- |
| Query           | 100 requests/minute |
| Document Upload | 50 requests/minute  |
| Graph Read      | 500 requests/minute |
| Graph Write     | 100 requests/minute |

---

## WebSocket (Future)

Real-time subscriptions for task updates:

```javascript
const ws = new WebSocket("ws://localhost:8080/api/v1/ws");

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log("Task update:", update);
};

ws.send(
  JSON.stringify({
    type: "subscribe",
    channel: "tasks",
    track_id: "task_20251224_143000_abc123",
  })
);
```

---

## SDK Examples

### TypeScript/JavaScript

```typescript
import { edgequakeApi } from "@/lib/api/edgequake";

// Upload document
const result = await edgequakeApi.uploadDocument({
  content: "Document text...",
  title: "My Document",
});

// Query
const response = await edgequakeApi.query({
  query: "What is the main topic?",
  mode: "hybrid",
});

// Streaming query
for await (const chunk of edgequakeApi.queryStream({
  query: "Explain in detail...",
  mode: "hybrid",
})) {
  console.log(chunk.content);
}
```

### Rust

```rust
use edgequake_core::EdgeQuake;

let eq = EdgeQuake::connect("http://localhost:8080").await?;

// Upload document
let doc_id = eq.upload_document("Document text...", "My Document").await?;

// Query
let response = eq.query("What is the main topic?", QueryMode::Hybrid).await?;
println!("Answer: {}", response.answer);
```
