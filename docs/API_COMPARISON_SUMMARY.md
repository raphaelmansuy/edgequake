# API Comparison Summary: EdgeQuake vs LightRAG

**Quick Reference Guide** | [Full Analysis](./API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md)

---

## 📊 Quick Stats

| Metric | EdgeQuake | LightRAG |
|--------|-----------|----------|
| **Total Endpoints** | 11 | 40+ |
| **Framework** | Axum (Rust) | FastAPI (Python) |
| **Authentication** | ❌ None | ✅ OAuth2 + API Key |
| **Multi-tenant** | ❌ No | ✅ Yes |
| **Async Processing** | ❌ Sync only | ✅ Background tasks |
| **Graph Editing** | ❌ Read-only | ✅ Full CRUD |

---

## ✅ Feature Parity Matrix

| Feature | EdgeQuake | LightRAG |
|---------|:---------:|:--------:|
| **Core RAG** | | |
| Health Check | ✅ | ✅ |
| Document Upload | ✅ | ✅ |
| Query (basic) | ✅ | ✅ |
| Streaming Query | ✅ | ✅ |
| Graph Visualization | ✅ | ✅ |
| **Advanced RAG** | | |
| Token Budget Control | ❌ | ✅ |
| Conversation History | ❌ | ✅ |
| Custom Keywords | ❌ | ✅ |
| Custom Prompts | ❌ | ✅ |
| Rerank Control | ⚠️ | ✅ |
| **Document Management** | | |
| File Upload | ❌ | ✅ |
| Direct Text Insert | ❌ | ✅ |
| Batch Insert | ❌ | ✅ |
| Status Tracking | ❌ | ✅ |
| Directory Scan | ❌ | ✅ |
| **Graph Management** | | |
| View Graph | ✅ | ✅ |
| Search Nodes | ✅ | ✅ |
| Create Entity | ❌ | ✅ |
| Edit Entity | ❌ | ✅ |
| Merge Entity | ❌ | ✅ |
| Create Relation | ❌ | ✅ |
| Edit Relation | ❌ | ✅ |
| **Production** | | |
| Authentication | ❌ | ✅ |
| Multi-tenancy | ❌ | ✅ |
| Admin APIs | ❌ | ✅ |
| Membership Mgmt | ❌ | ✅ |
| Ollama Proxy | ❌ | ✅ |

**Legend:** ✅ Available | ❌ Missing | ⚠️ Partial

---

## 🎯 Parity Score

```
Core Endpoints:       60% ████████▒▒▒▒▒▒
Advanced Features:    30% ████▒▒▒▒▒▒▒▒▒▒
Production Features:  10% █▒▒▒▒▒▒▒▒▒▒▒▒▒

Overall:             ~35% ████▒▒▒▒▒▒▒▒▒▒
```

---

## 🔍 Key Differences

### EdgeQuake Advantages
✅ **Performance:** Native Rust, lower memory, faster execution  
✅ **Type Safety:** Compile-time guarantees  
✅ **Concurrency:** Superior async handling (Tokio)  
✅ **Detailed Stats:** Query timing breakdown  

### LightRAG Advantages
✅ **Feature Complete:** 3.6x more endpoints  
✅ **Async Processing:** Background tasks with tracking  
✅ **Advanced Query:** Token budgets, conversation history  
✅ **Graph Editing:** Manual knowledge entry & correction  
✅ **Multi-tenant:** Full isolation & admin controls  
✅ **Authentication:** Production-ready security  

---

## 🚀 Quick Migration Guide

### ✅ Direct Compatible (No Changes)
```bash
# Health
GET /health → GET /health

# Query (basic)
POST /api/v1/query → POST /query
  query, mode → query, mode

# Graph
GET /api/v1/graph → GET /graphs
```

### ⚠️ Requires Adaptation
```bash
# Document Upload
POST /api/v1/documents (JSON)
  ↓ convert to ↓
POST /documents/upload (multipart/form-data)

# Advanced Query
Remove: token_budgets, conversation_history, keywords
Keep: query, mode, max_results
```

### ❌ Not Supported
- Multi-tenancy features
- Authentication headers
- Background task tracking
- Graph editing operations
- Admin functions

---

## 📋 EdgeQuake Roadmap Priorities

### Phase 1: Core RAG (Target: v1.1)
1. ✅ Background task processing + track_id
2. ✅ Document status tracking (pending/processing/indexed/failed)
3. ✅ Token budget controls (max_entity_tokens, max_relation_tokens)
4. ✅ Conversation history support
5. ✅ Direct text insertion endpoints

### Phase 2: Graph Management (Target: v1.2)
6. ✅ Entity create/edit/merge endpoints
7. ✅ Relationship create/edit endpoints
8. ✅ Bulk operations (delete all, delete failed)
9. ✅ Directory scanning

### Phase 3: Production (Target: v2.0)
10. ✅ JWT authentication
11. ✅ Multi-tenancy (optional)
12. ✅ OpenTelemetry + Prometheus
13. ✅ Rate limiting

---

## 💡 Recommendations

### Choose EdgeQuake If:
- Performance & efficiency are critical
- Resource constraints (memory, CPU)
- Type safety is important
- Basic RAG features sufficient

### Choose LightRAG If:
- Feature richness required
- Multi-tenancy needed
- Faster customization desired
- Python ecosystem advantages

### Use Both If:
- EdgeQuake for performance-critical queries
- LightRAG for admin & management UI
- Hybrid architecture with shared storage

---

## 📖 Example Endpoint Comparisons

### Document Upload

**EdgeQuake:**
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "AI research paper...", "title": "GPT-4"}'
```

**LightRAG:**
```bash
curl -X POST http://localhost:8020/documents/upload \
  -F "file=@paper.pdf"
# Returns track_id for async status polling
```

### Advanced Query

**EdgeQuake:**
```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is RAG?", "mode": "hybrid"}'
```

**LightRAG:**
```bash
curl -X POST http://localhost:8020/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is RAG?",
    "mode": "hybrid",
    "max_entity_tokens": 1000,
    "max_relation_tokens": 1000,
    "conversation_history": [
      {"role": "user", "content": "Tell me about AI"},
      {"role": "assistant", "content": "AI is..."}
    ],
    "hl_keywords": ["artificial intelligence", "machine learning"]
  }'
```

---

## 🔗 Related Documentation

- [Full API Comparison](./API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md) - Detailed 200+ line analysis
- [EdgeQuake API Reference](./0003-api-reference.md)
- [LightRAG API Docs](../lightrag/api/) - Python implementation

---

**Last Updated:** January 2025  
**Version:** 1.0  
**Status:** EdgeQuake 0.1.0 vs LightRAG Latest
