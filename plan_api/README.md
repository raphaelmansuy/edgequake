# EdgeQuake API v2.0 Specification

**Complete API specification for incorporating all LightRAG features into EdgeQuake**

---

## 📋 Table of Contents

| Document | Description | LightRAG Reference | Size |
|----------|-------------|-------------------|------|
| [00-MASTER_PLAN.md](00-MASTER_PLAN.md) | Comprehensive 18-month roadmap and architecture | Overall comparison | 20KB |
| [01-background-tasks.md](01-background-tasks.md) | Async task processing with Redis/Channels | `document_routes.py`, `utils.py` | 27KB |
| [02-document-enhancements.md](02-document-enhancements.md) | File uploads, status tracking, deduplication | `document_routes.py`, `utils.py` | 19KB |
| [03-advanced-query.md](03-advanced-query.md) | Token budgets, conversation history, bypass mode | `query_routes.py`, `base.py` | 24KB |
| [04-graph-management.md](04-graph-management.md) | Entity/Relationship CRUD, merge operations | `graph_routes.py`, `kg/*.py` | 22KB |
| [05-authentication.md](05-authentication.md) | JWT + API key authentication, RBAC | `auth.py`, `utils_api.py` | 15KB |
| [06-multi-tenancy.md](06-multi-tenancy.md) | Tenant isolation, workspaces, memberships | `tenant_routes.py`, `tenant_rag_manager.py` | 12KB |
| [07-api-reference.md](07-api-reference.md) | Complete API reference (59 endpoints) | All route files | 8KB |
| [08-observability.md](08-observability.md) | Metrics, tracing, logging, dashboards | `utils.py` (logging) | 14KB |
| [09-migration-guide.md](09-migration-guide.md) | Step-by-step migration from v1.0 → v2.0 | N/A | 18KB |
| [10-implementation-checklist.md](10-implementation-checklist.md) | Detailed implementation tracking | N/A | 12KB |
| [11-lightrag-cross-reference.md](11-lightrag-cross-reference.md) | **Detailed LightRAG code mapping** | **All files** | **20KB** |

**Total:** ~211KB of comprehensive specifications with full LightRAG cross-references

---

## 🎯 Quick Overview

### Current State (EdgeQuake v1.0.0)
- **Endpoints:** 11
- **Features:** Basic RAG, read-only graph
- **Tech:** Rust/Axum, PostgreSQL AGE

### Target State (EdgeQuake v2.0.0)
- **Endpoints:** 59 (+48 new)
- **Features:** Full LightRAG parity
- **Timeline:** 18 months, 3 major releases

---

## 🚀 Three-Phase Roadmap

### Phase 1: Background Tasks (v1.1.0)
**Timeline:** Q4 2025 / Q1 2026 (3-4 months)  
**Endpoints:** +8 (11 → 19)

**Key Features:**
- ✅ Async task processing (tokio channels + Redis)
- ✅ Document status tracking (pending/processing/indexed/failed)
- ✅ Token budget controls (cost management)
- ✅ Conversation history support
- ✅ File upload with multipart/form-data
- ✅ Content deduplication (SHA-256)

**Read:** [01-background-tasks.md](01-background-tasks.md), [02-document-enhancements.md](02-document-enhancements.md), [03-advanced-query.md](03-advanced-query.md)

---

### Phase 2: Graph Management (v1.2.0)
**Timeline:** Q2 2026 (4-6 months)  
**Endpoints:** +15 (19 → 34)

**Key Features:**
- ✅ Entity CRUD (create, read, update, delete, merge)
- ✅ Relationship CRUD
- ✅ Bulk operations (clear, scan directory)
- ✅ Graph analytics (statistics, popular labels)
- ✅ Audit logging (track manual changes)

**Read:** [04-graph-management.md](04-graph-management.md)

---

### Phase 3: Production Features (v2.0.0)
**Timeline:** Q3-Q4 2026 (5-6 months)  
**Endpoints:** +25 (34 → 59)

**Key Features:**
- ✅ JWT authentication + API keys
- ✅ Multi-tenancy with tenant isolation
- ✅ RBAC (admin/user/readonly)
- ✅ OpenTelemetry + Prometheus observability
- ✅ Rate limiting
- ✅ Admin APIs

**Read:** [05-authentication.md](05-authentication.md), [06-multi-tenancy.md](06-multi-tenancy.md), [08-observability.md](08-observability.md)

---

## 📊 Endpoint Summary

| Category | v1.0 | v1.1 | v1.2 | v2.0 |
|----------|------|------|------|------|
| Health | 4 | 4 | 4 | 4 |
| Documents | 3 | 11 | 14 | 14 |
| Query | 2 | 3 | 3 | 3 |
| Tasks | 0 | 5 | 5 | 5 |
| Graph | 2 | 2 | 17 | 17 |
| Auth | 0 | 0 | 0 | 3 |
| Multi-Tenant | 0 | 0 | 0 | 12 |
| Admin | 0 | 0 | 0 | 3 |
| **Total** | **11** | **19** | **34** | **59** |

**Full list:** [07-api-reference.md](07-api-reference.md)

---

## 🏗️ Architecture Highlights

### Task Processing (Phase 1)

```
User Request → Task Queue → Worker Pool → Status Updates
     ↓              ↓             ↓              ↓
   202 Accepted   Redis/    Background      Polling via
   + track_id    Channels    Workers       GET /tasks/{id}
```

### Graph CRUD (Phase 2)

```
Manual Operations → EntityManager → AGE Graph
                         ↓
                    Audit Log
                    (is_manual=true)
```

### Multi-Tenancy (Phase 3)

```
Tenant
  └── Workspaces (Knowledge Bases)
       └── Documents → Entities → Relationships
       
Isolation: {tenant_id}:{workspace_id}:doc-{uuid}
```

---

## 🔧 Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| **Web Framework** | Axum | 0.8.8 |
| **Async Runtime** | Tokio | 1.43+ |
| **Graph Database** | PostgreSQL + AGE | 15+ |
| **Vector Storage** | Qdrant / Milvus | TBD |
| **Task Queue** | Redis (optional) | 7.0+ |
| **Authentication** | jsonwebtoken | 9.0+ |
| **Observability** | OpenTelemetry | 0.20+ |
| **Metrics** | Prometheus | 0.13+ |
| **Tracing** | Jaeger / Tempo | Latest |

---

## 📖 How to Use This Specification

### For Product Managers
1. Start with [00-MASTER_PLAN.md](00-MASTER_PLAN.md) - understand vision and timeline
2. Review [07-api-reference.md](07-api-reference.md) - see complete API surface
3. Read phase-specific docs for feature details

### For Developers
1. Read [00-MASTER_PLAN.md](00-MASTER_PLAN.md) - architecture overview
2. **Read [11-lightrag-cross-reference.md](11-lightrag-cross-reference.md)** - detailed LightRAG code mapping
3. Choose a phase to implement
4. Read phase-specific docs (e.g., 01, 02, 03 for Phase 1)
5. Follow [10-implementation-checklist.md](10-implementation-checklist.md)
6. Use [09-migration-guide.md](09-migration-guide.md) for database migrations
7. **Cross-reference LightRAG source code** for implementation details

### For DevOps Engineers
1. Read [08-observability.md](08-observability.md) - monitoring setup
2. Read [09-migration-guide.md](09-migration-guide.md) - deployment steps
3. Review [06-multi-tenancy.md](06-multi-tenancy.md) - tenant configuration

### Key Implementation References

**Critical:** Always refer to [11-lightrag-cross-reference.md](11-lightrag-cross-reference.md) for:
- Exact LightRAG source file locations
- Python → Rust translation patterns
- Implementation priority matrix
- Testing strategy mapping
- Configuration cross-references

---

## ✅ Implementation Status

```markdown
- [x] Master plan created
- [x] Phase 1 specifications complete
- [x] Phase 2 specifications complete
- [x] Phase 3 specifications complete
- [x] API reference complete
- [x] Migration guide complete
- [x] Implementation checklist complete
- [ ] Begin Phase 1 implementation
- [ ] Begin Phase 2 implementation
- [ ] Begin Phase 3 implementation
```

---

## 🎯 Success Metrics

### Phase 1 (v1.1.0)
- ✅ 95%+ async operations (no blocking)
- ✅ <500ms p95 query latency
- ✅ 90%+ test coverage
- ✅ Zero breaking changes

### Phase 2 (v1.2.0)
- ✅ 100% graph CRUD coverage
- ✅ <100ms entity merge operation
- ✅ Full audit trail
- ✅ 90%+ test coverage

### Phase 3 (v2.0.0)
- ✅ 99.9% uptime
- ✅ 1000 RPS sustained load
- ✅ Complete tenant isolation
- ✅ <200ms p95 latency
- ✅ Security audit passed

---

## 🔗 Related Documents

- **LightRAG Cross-Reference:** [11-lightrag-cross-reference.md](11-lightrag-cross-reference.md) - **NEW** Detailed mapping to LightRAG source code
- **API Comparison:** [../docs/API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md](../docs/API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md) - Feature gap analysis
- **Current Implementation:** [../edgequake/](../edgequake/)
- **LightRAG Reference:** [../lightrag/](../lightrag/)
- **Production Docs:** [../docs/](../docs/)
- **Legacy Implementation:** [../lightrag/api/](../lightrag/api/)

---

## 📞 Getting Started

```bash
# Read the master plan
cat plan_api/00-MASTER_PLAN.md

# Review Phase 1 specifications
cat plan_api/01-background-tasks.md
cat plan_api/02-document-enhancements.md
cat plan_api/03-advanced-query.md

# Start implementation
cd edgequake/
cargo build
cargo test
```

---

## 🤝 Contributing

When implementing features from this specification:

1. **Follow the order:** Phase 1 → Phase 2 → Phase 3
2. **Check dependencies:** See "Dependencies" section in each spec
3. **Write tests:** 90%+ coverage required
4. **Update docs:** Keep OpenAPI spec in sync
5. **Track progress:** Update [10-implementation-checklist.md](10-implementation-checklist.md)

---

## 📅 Timeline Summary

| Phase | Release | Duration | Start | End |
|-------|---------|----------|-------|-----|
| Phase 1 | v1.1.0 | 3-4 months | Dec 2025 | Mar 2026 |
| Phase 2 | v1.2.0 | 4-6 months | Apr 2026 | Sep 2026 |
| Phase 3 | v2.0.0 | 5-6 months | Oct 2026 | Mar 2027 |
| **Total** | - | **18 months** | Dec 2025 | Jun 2027 |

---

## 📝 Document Stats

- **Total Documents:** 11
- **Total Size:** ~191KB
- **Total Endpoints Specified:** 59
- **Code Examples:** 100+
- **Database Migrations:** 7
- **Test Cases:** 200+

---

**Status:** ✅ Specification Complete  
**Version:** 1.0  
**Last Updated:** December 22, 2025  
**Ready for Implementation:** YES

---

## 🚦 Next Steps

1. ✅ Review and approve specifications
2. ✅ Set up development environment
3. 🔄 Begin Phase 1 implementation
4. 🔄 Weekly progress reviews
5. 🔄 Monthly milestone check-ins

**Let's build EdgeQuake v2.0! 🚀**
