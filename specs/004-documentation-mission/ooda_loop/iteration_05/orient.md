# OODA Iteration 05 - Orient

**Date**: 2025-01-XX
**Focus**: REST API Reference Documentation

## 🧭 Orientation

### 1. Target Audience

| Audience              | Needs                                               |
| --------------------- | --------------------------------------------------- |
| Frontend developers   | Quick endpoint reference, request/response examples |
| Integration engineers | Authentication, error handling, rate limits         |
| DevOps                | Health checks, metrics endpoints                    |
| API consumers         | OpenAPI/Swagger reference                           |

### 2. Documentation Strategy

Create a single comprehensive REST API reference that covers:

- Authentication (JWT + API Key)
- Core endpoints with examples
- Error handling patterns
- Rate limiting info

Rather than multiple files, consolidate into one well-organized reference.

### 3. Key Patterns to Document

```
┌─────────────────────────────────────────────────────────────────┐
│                    API REQUEST FLOW                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Client                                                          │
│    │                                                             │
│    ├─▶ Headers                                                   │
│    │   ├─ Authorization: Bearer <token>                         │
│    │   ├─ X-Tenant-ID: <tenant>                                 │
│    │   └─ X-Workspace-ID: <workspace>                           │
│    │                                                             │
│    ├─▶ Request Body (JSON)                                      │
│    │                                                             │
│    └─▶ Response                                                  │
│        ├─ Success: 2xx + JSON data                              │
│        └─ Error: 4xx/5xx + RFC 7807 problem details            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 4. Endpoint Categories to Document

**Document in Detail**:

1. Documents (POST, GET, DELETE)
2. Query (POST /query, POST /query/stream)
3. Chat (POST /chat/completions)
4. Graph (GET /graph, entities, relationships)

**Summarize**: 5. Workspaces 6. Conversations 7. Tasks 8. Health/Metrics

### 5. Example Format

For each endpoint:

- Method + Path
- Description
- Headers
- Request body (JSON schema)
- Response body (JSON example)
- cURL example

### 6. Error Response Format

RFC 7807 Problem Details:

```json
{
  "type": "https://edgequake.dev/errors/not-found",
  "title": "Resource Not Found",
  "status": 404,
  "detail": "Document with ID 'doc123' not found",
  "instance": "/api/v1/documents/doc123"
}
```
