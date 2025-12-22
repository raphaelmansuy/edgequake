# LightRAG Implementation Cross-Reference

**Purpose:** Map EdgeQuake API v2.0 specifications to LightRAG source code  
**Last Updated:** December 22, 2025

---

## Document Cross-Reference Map

### Phase 1 Specifications

| EdgeQuake Spec                                             | LightRAG Source Files                                                                                                                                                                                                         | Key Implementation Details                                                                                                                                                                                                                                                |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [01-background-tasks.md](01-background-tasks.md)           | `lightrag/api/routers/document_routes.py`<br>`lightrag/utils.py:generate_track_id()`<br>`lightrag/base.py:DocStatus`                                                                                                          | Track ID format: `{type}-{uuid}`<br>Background task processing with FastAPI BackgroundTasks<br>Document status enum: ready, processing, error                                                                                                                             |
| [02-document-enhancements.md](02-document-enhancements.md) | `lightrag/api/routers/document_routes.py:upload_document()`<br>`lightrag/api/routers/document_routes.py:insert_text()`<br>`lightrag/api/routers/document_routes.py:insert_texts()`<br>`lightrag/utils.py:compute_mdhash_id()` | File upload with multipart/form-data<br>SHA-256 hashing via `compute_mdhash_id()`<br>Duplicate detection in doc_status<br>Docling for PDF/Word parsing<br>Path traversal sanitization                                                                                     |
| [03-advanced-query.md](03-advanced-query.md)               | `lightrag/api/routers/query_routes.py:QueryRequest`<br>`lightrag/base.py:QueryParam`<br>`lightrag/lightrag.py:query()`                                                                                                        | Token budgets: `max_entity_tokens`, `max_relation_tokens`, `max_total_tokens`<br>Conversation history: `List[Dict]` format<br>Keyword extraction: `hl_keywords`, `ll_keywords`<br>Bypass mode: Direct LLM without RAG<br>`only_need_context` and `only_need_prompt` flags |

### Phase 2 Specifications

| EdgeQuake Spec                                   | LightRAG Source Files                                                                                   | Key Implementation Details                                                                                                                                                                                           |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [04-graph-management.md](04-graph-management.md) | `lightrag/api/routers/graph_routes.py`<br>`lightrag/kg/neo4j_impl.py`<br>`lightrag/kg/networkx_impl.py` | Entity CRUD: create, update, delete, merge<br>Relationship CRUD<br>Entity merge: `entities_to_change` + `entity_to_change_into`<br>Merge strategies: prefer_target, prefer_source<br>Manual entity flag: `is_manual` |

### Phase 3 Specifications

| EdgeQuake Spec                               | LightRAG Source Files                                                                                                                                             | Key Implementation Details                                                                                                                                                                           |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [05-authentication.md](05-authentication.md) | `lightrag/api/auth.py:AuthHandler`<br>`lightrag/api/utils_api.py:get_combined_auth_dependency()`                                                                  | JWT tokens: `sub`, `exp`, `role`, `metadata`<br>API key authentication<br>Token expiration: default vs guest<br>Role-based: admin, user, guest<br>OAuth2 scheme                                      |
| [06-multi-tenancy.md](06-multi-tenancy.md)   | `lightrag/api/routers/tenant_routes.py`<br>`lightrag/models/tenant.py:TenantContext`<br>`lightrag/services/tenant_service.py`<br>`lightrag/tenant_rag_manager.py` | Tenant hierarchy: Tenant → KB (workspace) → Documents<br>Tenant context: `tenant_id`, `kb_id`, `user_id`, `role`<br>Permission checking: OWNER, ADMIN, EDITOR, VIEWER<br>Tenant isolation in storage |
| [08-observability.md](08-observability.md)   | `lightrag/utils.py:logger`<br>External integration points                                                                                                         | Structured logging with python logging<br>Request tracking<br>Error tracebacks with ascii_colors                                                                                                     |

---

## Detailed Code References

### 1. Background Tasks & Track IDs

**LightRAG Implementation:**

```python
# File: lightrag/utils.py
def generate_track_id(prefix: str = "track") -> str:
    """Generate a unique tracking ID"""
    return f"{prefix}-{uuid.uuid4().hex[:8]}"

# File: lightrag/api/routers/document_routes.py
@router.post("/upload")
async def upload_document(
    file: UploadFile,
    background_tasks: BackgroundTasks,
    ...
):
    track_id = generate_track_id("upload")
    background_tasks.add_task(process_document, ...)
    return {"track_id": track_id, "status": "pending"}
```

**EdgeQuake Mapping:**

- See [01-background-tasks.md#Track-ID-Generation](01-background-tasks.md#track-id-generation)
- Implement in Rust: `format!("{}-{}", type, uuid::Uuid::new_v4())`
- Use tokio channels or Redis for queue

### 2. Document Status Tracking

**LightRAG Implementation:**

```python
# File: lightrag/base.py
class DocStatus(str, Enum):
    READY = "ready"
    PROCESSING = "processing"
    ERROR = "error"

# File: lightrag/api/routers/document_routes.py
doc_status: Dict[str, DocProcessingStatus] = {}
```

**EdgeQuake Mapping:**

- See [02-document-enhancements.md#Document-Status-Schema](02-document-enhancements.md#document-status-schema)
- PostgreSQL table instead of in-memory dict
- Additional fields: chunks_processed, entities_extracted, relationships_extracted

### 3. Content Deduplication

**LightRAG Implementation:**

```python
# File: lightrag/utils.py
def compute_mdhash_id(content: str, prefix: str = "doc") -> str:
    """Compute MD5 hash for content deduplication"""
    return f"{prefix}-{hashlib.md5(content.encode()).hexdigest()}"

# File: lightrag/api/routers/document_routes.py
doc_id = compute_mdhash_id(content)
if doc_id in doc_status:
    return {"status": "duplicated", "doc_id": doc_id}
```

**EdgeQuake Mapping:**

- See [02-document-enhancements.md#Content-Deduplication](02-document-enhancements.md#content-deduplication)
- Use SHA-256 instead of MD5 (more secure)
- Store content_hash in document_status table
- Return 409 Conflict with existing document info

### 4. Token Budget Controls

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/query_routes.py
class QueryRequest(BaseModel):
    max_entity_tokens: Optional[int] = Field(
        default=None,
        description="Maximum tokens for entity context",
        ge=1,
    )
    max_relation_tokens: Optional[int] = Field(
        default=None,
        description="Maximum tokens for relationship context",
        ge=1,
    )
    max_total_tokens: Optional[int] = Field(
        default=None,
        description="Maximum total tokens budget",
        ge=1,
    )

# File: lightrag/lightrag.py (query method)
# Token budget enforcement happens during context retrieval
```

**EdgeQuake Mapping:**

- See [03-advanced-query.md#Token-Budget-Control](03-advanced-query.md#token-budget-control)
- Implement TokenBudgetController trait
- Use tiktoken or similar for token counting
- Truncate context when budget exceeded

### 5. Conversation History

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/query_routes.py
class QueryRequest(BaseModel):
    conversation_history: Optional[List[Dict[str, Any]]] = Field(
        default=None,
        description="Past conversation history",
    )

# Format: [{'role': 'user/assistant', 'content': 'message'}]
```

**EdgeQuake Mapping:**

- See [03-advanced-query.md#Conversation-History](03-advanced-query.md#conversation-history)
- PostgreSQL table: conversation_history (session_id, role, content, created_at)
- ConversationHistoryManager for retrieval
- Include in LLM context

### 6. Keyword Extraction

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/query_routes.py
class QueryRequest(BaseModel):
    hl_keywords: list[str] = Field(
        default_factory=list,
        description="High-level keywords (leave empty for LLM generation)",
    )
    ll_keywords: list[str] = Field(
        default_factory=list,
        description="Low-level keywords (leave empty for LLM generation)",
    )

# If empty, LLM extracts keywords automatically
```

**EdgeQuake Mapping:**

- See [03-advanced-query.md#Keyword-Extraction](03-advanced-query.md#keyword-extraction)
- KeywordExtractor with LLM-based extraction
- Two-tier: high-level (concepts) + low-level (specific terms)
- Cache extracted keywords per query

### 7. Entity CRUD Operations

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/graph_routes.py
class EntityCreateRequest(BaseModel):
    entity_name: str
    entity_data: Dict[str, Any]

class EntityUpdateRequest(BaseModel):
    entity_name: str
    updated_data: Dict[str, Any]
    allow_rename: bool = False
    allow_merge: bool = False

class EntityMergeRequest(BaseModel):
    entities_to_change: list[str]  # Sources
    entity_to_change_into: str      # Target

# Routes: POST /entities, PUT /entities/{name}, DELETE /entities/{name}
```

**EdgeQuake Mapping:**

- See [04-graph-management.md#Entity-Operations](04-graph-management.md#entity-operations)
- AGE Cypher queries for CRUD
- Audit logging for all operations
- Add is_manual flag to distinguish manual vs extracted

### 8. Entity Merge Logic

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/graph_routes.py
@router.post("/entities/merge")
async def merge_entities(request: EntityMergeRequest):
    # 1. Transfer all relationships from source entities to target
    # 2. Delete source entities
    # 3. Update entity properties (prefer longer descriptions)
```

**EdgeQuake Mapping:**

- See [04-graph-management.md#Entity-Merge](04-graph-management.md#entity-merge)
- Merge strategies: prefer_target, prefer_source, concatenate, longer
- EntityMerger trait with strategy pattern
- Transaction support for atomicity

### 9. JWT Authentication

**LightRAG Implementation:**

```python
# File: lightrag/api/auth.py
class AuthHandler:
    def create_token(self, username: str, role: str = "user") -> str:
        expire = datetime.utcnow() + timedelta(hours=self.expire_hours)
        payload = TokenPayload(sub=username, exp=expire, role=role)
        return jwt.encode(payload.dict(), self.secret, algorithm=self.algorithm)

    def validate_token(self, token: str) -> dict:
        payload = jwt.decode(token, self.secret, algorithms=[self.algorithm])
        # Check expiration
        return payload

# File: lightrag/api/utils_api.py
async def get_combined_auth_dependency():
    # Try JWT Bearer token
    # Fallback to API key (X-API-Key header)
```

**EdgeQuake Mapping:**

- See [05-authentication.md#JWT-Authentication](05-authentication.md#jwt-authentication)
- Use `jsonwebtoken` crate for Rust
- Claims: sub (user_id), username, role, exp, iat
- AuthUser extractor for Axum
- ApiKeyAuth extractor for API keys

### 10. Multi-Tenancy Architecture

**LightRAG Implementation:**

```python
# File: lightrag/models/tenant.py
class TenantContext:
    tenant_id: str
    kb_id: str
    user_id: str
    role: UserRole

class Permission(str, Enum):
    READ = "read"
    WRITE = "write"
    DELETE = "delete"
    ADMIN = "admin"

# File: lightrag/api/dependencies.py
async def get_tenant_context(
    x_tenant_id: str = Header(...),
    x_kb_id: str = Header(...),
) -> TenantContext:
    # Validate tenant + KB access
    # Check user permissions
    return TenantContext(...)

# File: lightrag/tenant_rag_manager.py
class TenantRAGManager:
    def get_rag(self, tenant_id: str, kb_id: str) -> LightRAG:
        # Return isolated RAG instance for tenant+KB
```

**EdgeQuake Mapping:**

- See [06-multi-tenancy.md#Tenant-Context](06-multi-tenancy.md#tenant-context)
- Middleware for tenant injection
- Headers: X-Tenant-ID, X-Workspace-ID (KB)
- Storage isolation via namespace: {tenant_id}:{workspace_id}:doc-{uuid}
- Feature flag for optional multi-tenancy

### 11. User Roles & Permissions

**LightRAG Implementation:**

```python
# File: lightrag/api/models.py
class UserRole(str, Enum):
    OWNER = "owner"
    ADMIN = "admin"
    EDITOR = "editor"
    VIEWER = "viewer"

# File: lightrag/api/dependencies.py
async def check_permission(
    context: TenantContext,
    required_permission: Permission,
):
    # OWNER/ADMIN: all permissions
    # EDITOR: read + write
    # VIEWER: read only
```

**EdgeQuake Mapping:**

- See [05-authentication.md#Role-Based-Access-Control](05-authentication.md#role-based-access-control)
- Simplified: admin, user, readonly
- Permission checker function
- Role guards on endpoints

### 12. Pagination Pattern

**LightRAG Implementation:**

```python
# File: lightrag/api/routers/tenant_routes.py
class PaginatedKBResponse(BaseModel):
    items: List[KBResponse]
    total: int
    page: int
    page_size: int
    total_pages: int
    has_next: bool
    has_prev: bool

@router.get("/knowledge-bases")
async def list_kbs(
    page: int = Query(1, ge=1),
    page_size: int = Query(10, ge=1, le=100),
):
    offset = (page - 1) * page_size
    items = db.query().offset(offset).limit(page_size)
    total = db.count()
    return PaginatedKBResponse(
        items=items,
        total=total,
        page=page,
        page_size=page_size,
        total_pages=(total + page_size - 1) // page_size,
        has_next=page * page_size < total,
        has_prev=page > 1,
    )
```

**EdgeQuake Mapping:**

- Apply to: /documents, /tasks, /workspaces, /memberships
- Query params: page (1-indexed), page_size (default 10, max 100)
- Response includes: items, total, page, page_size, has_next, has_prev

---

## Implementation Priority Matrix

| Feature              | LightRAG File      | EdgeQuake Priority | Complexity | Dependencies |
| -------------------- | ------------------ | ------------------ | ---------- | ------------ |
| Track ID generation  | utils.py           | HIGH               | LOW        | None         |
| Background tasks     | document_routes.py | HIGH               | MEDIUM     | Tokio/Redis  |
| Document status      | base.py            | HIGH               | LOW        | PostgreSQL   |
| Content hashing      | utils.py           | HIGH               | LOW        | SHA-256      |
| Token budgets        | query_routes.py    | MEDIUM             | MEDIUM     | Tokenizer    |
| Conversation history | query_routes.py    | MEDIUM             | MEDIUM     | PostgreSQL   |
| Keyword extraction   | query_routes.py    | LOW                | MEDIUM     | LLM          |
| Entity CRUD          | graph_routes.py    | MEDIUM             | HIGH       | AGE          |
| Entity merge         | graph_routes.py    | LOW                | HIGH       | AGE          |
| JWT auth             | auth.py            | HIGH               | MEDIUM     | jsonwebtoken |
| API keys             | auth.py            | MEDIUM             | LOW        | PostgreSQL   |
| Multi-tenancy        | tenant_routes.py   | LOW                | HIGH       | Many         |
| RBAC                 | dependencies.py    | MEDIUM             | MEDIUM     | Auth         |

---

## Testing Strategy Cross-Reference

### LightRAG Test Patterns

```python
# Unit tests
tests/test_utils.py
tests/test_auth.py

# Integration tests
tests/test_api.py

# E2E tests
tests/test_workflows.py
```

### EdgeQuake Test Plan

See [10-implementation-checklist.md#Testing](10-implementation-checklist.md#testing)

---

## Configuration Cross-Reference

### LightRAG Environment Variables

```bash
# File: lightrag/api/config.py
TOKEN_SECRET=secret_key
JWT_ALGORITHM=HS256
TOKEN_EXPIRE_HOURS=24
GUEST_TOKEN_EXPIRE_HOURS=1
AUTH_ACCOUNTS=admin:password,user:password
ENABLE_MULTI_TENANT=false
```

### EdgeQuake Configuration

See [05-authentication.md#Configuration](05-authentication.md#configuration)
See [06-multi-tenancy.md#Configuration](06-multi-tenancy.md#configuration)

---

**Related Documents:**

- [API Comparison](../docs/API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md) - Feature gap analysis
- [00-MASTER_PLAN.md](00-MASTER_PLAN.md) - Overall roadmap
- [10-implementation-checklist.md](10-implementation-checklist.md) - Task tracking

**Last Updated:** December 22, 2025  
**Status:** ✅ Complete
