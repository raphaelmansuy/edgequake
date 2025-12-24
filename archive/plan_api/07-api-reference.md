# Complete API Reference (v2.0)

**Version:** 2.0.0  
**Last Updated:** December 22, 2025  
**Base URL:** `http://localhost:8080/api/v1`

---

## Quick Index

### Health & Status
- `GET /health` - Health check
- `GET /ready` - Readiness probe
- `GET /live` - Liveness probe
- `GET /metrics` - Prometheus metrics

### Authentication
- `POST /auth/token` - Login (JWT)
- `POST /auth/refresh` - Refresh token
- `POST /auth/logout` - Logout

### Documents (14 endpoints)
- `POST /documents` - Upload document (JSON text)
- `POST /documents/upload` - Upload file (multipart)
- `POST /documents/text` - Insert text
- `POST /documents/texts` - Insert multiple texts
- `POST /documents/scan` - Scan directory
- `GET /documents` - List documents
- `GET /documents/status` - Get document status
- `GET /documents/{id}` - Get document
- `GET /documents/stats` - Statistics
- `DELETE /documents/{id}` - Delete document
- `DELETE /documents/file/{filename}` - Delete by filename
- `DELETE /documents/clear` - Delete all
- `DELETE /documents/failed` - Delete failed only
- `POST /documents/reindex-failed` - Reindex failed

### Query (3 endpoints)
- `POST /query` - Execute query
- `POST /query/stream` - Streaming query
- `POST /query/context` - Context only (no generation)

### Tasks (5 endpoints)
- `GET /tasks/{track_id}` - Get task status
- `GET /tasks` - List tasks
- `POST /tasks/{track_id}/cancel` - Cancel task
- `POST /tasks/{track_id}/retry` - Retry failed task

### Graph (15 endpoints)
- `GET /graph` - Get knowledge graph
- `GET /graph/nodes/{id}` - Get node
- `GET /graph/labels` - List labels
- `GET /graph/labels/popular` - Popular labels
- `GET /graph/labels/search` - Search labels
- `GET /graph/statistics` - Graph statistics
- `POST /graph/entities` - Create entity
- `GET /graph/entities/{id}` - Get entity
- `PUT /graph/entities/{id}` - Update entity
- `DELETE /graph/entities/{id}` - Delete entity
- `GET /graph/entities/exists` - Check existence
- `POST /graph/entities/merge` - Merge entities
- `POST /graph/relationships` - Create relationship
- `PUT /graph/relationships/{id}` - Update relationship
- `DELETE /graph/relationships/{id}` - Delete relationship

### Multi-Tenancy (12 endpoints)
- `GET /tenants` - List tenants
- `GET /tenants/me` - Current tenant
- `POST /tenants` - Create tenant
- `POST /tenants/select` - Select tenant
- `GET /workspaces` - List workspaces
- `GET /workspaces/{id}` - Get workspace
- `POST /workspaces` - Create workspace
- `PUT /workspaces/{id}` - Update workspace
- `DELETE /workspaces/{id}` - Delete workspace
- `POST /memberships` - Add membership
- `GET /memberships/{tenant_id}` - List members
- `GET /users/me/tenants` - My tenants

### Admin (3 endpoints)
- `GET /admin/stats` - System statistics
- `POST /admin/tenants` - Admin create tenant
- `GET /admin/tenants` - Admin list tenants

---

## Authentication

All endpoints (except `/health`, `/ready`, `/live`) require authentication in v2.0:

**JWT Bearer Token:**
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**API Key:**
```http
X-API-Key: sk_live_abc123def456...
```

**Multi-Tenant Headers:**
```http
X-Tenant-ID: tenant-123
X-Workspace-ID: workspace-456
```

---

## Full Endpoint Count

| Category | Endpoints | Notes |
|----------|-----------|-------|
| Health | 4 | Status checks + metrics |
| Auth | 3 | JWT authentication |
| Documents | 14 | Full document lifecycle |
| Query | 3 | Query execution |
| Tasks | 5 | Background task management |
| Graph | 15 | Knowledge graph CRUD |
| Multi-Tenancy | 12 | Tenant/workspace/membership |
| Admin | 3 | Admin operations |
| **Total** | **59** | **Complete API surface** |

---

## Response Codes

| Code | Meaning | Usage |
|------|---------|-------|
| 200 | OK | Successful GET/PUT/DELETE |
| 201 | Created | Successful POST (resource created) |
| 202 | Accepted | Async operation queued |
| 204 | No Content | Successful DELETE (no body) |
| 400 | Bad Request | Invalid input |
| 401 | Unauthorized | Missing/invalid authentication |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Duplicate resource |
| 413 | Payload Too Large | File/content too large |
| 422 | Unprocessable Entity | Validation failed |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | System overloaded |

---

## OpenAPI Specification

Full OpenAPI 3.1 spec available at:
- **JSON:** `http://localhost:8080/api-docs/openapi.json`
- **YAML:** `http://localhost:8080/api-docs/openapi.yaml`
- **Swagger UI:** `http://localhost:8080/swagger-ui`
- **ReDoc:** `http://localhost:8080/redoc`

---

**Status:** ✅ Complete API Reference  
**Total Endpoints:** 59  
**Version:** 2.0.0
