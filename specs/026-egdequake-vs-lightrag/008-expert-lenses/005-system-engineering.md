# 005 — System Engineering Lens

**Cross-ref:** [006 Robustness](../006-robustness/001-robustness-comparison.md) · [003 Ingestion](../003-ingestion/001-ingestion-comparison.md)

**Finding:** C-05

---

## Deployment Architecture

### LightRAG

```text
  Typical dev deploy:
  ─────────────────

  pip install lightrag
       │
       ├── JSON KV files (./rag_storage/)
       ├── NetworkX graph (in-memory)
       ├── NanoVectorDB / FAISS
       └── optional FastAPI (lightrag/api/)

  Production deploy (advanced):
  ────────────────────────────

  PostgreSQL + pgvector + Neo4j/Memgraph
  + Redis doc status
  + MinerU/Docling sidecars
```

**Complexity:** Flexible but **operator burden** — many moving parts, no single golden path.

### EdgeQuake

```text
  Production deploy (mandatory):
  ────────────────────────────

  Docker PostgreSQL (AGE + pgvector)
       │
       ├── Axum API (:8080)
       ├── WorkerPool (in-process)
       ├── Ollama / OpenAI providers
       └── Next.js WebUI (:3000)

  make dev-bg → single command stack
```

**Complexity:** **Opinionated** — fewer choices, clearer ops path.

---

## Service Health

| Signal | LightRAG | EdgeQuake |
|--------|:--------:|:---------:|
| `/health` endpoint | △ basic | ✓ component matrix |
| Storage mode in health | ✗ | ✓ postgresql |
| LLM provider name | △ | ✓ |
| Queue depth | △ pipeline status | ✓ metrics |
| Task pressure | ✗ | ✓ `task_queue_pressure.rs` |
| Startup recovery | △ | ✓ orphan tasks/docs |

EdgeQuake `handlers/health.rs` — operator-grade.

---

## Task Queue Design

```text
  LightRAG pipeline queues          EdgeQuake WorkerPool
  ────────────────────────          ────────────────────

  parse_queue (sized)               Postgres tasks table
  analyze_queue (sized)             + in-memory channel
  insert_queue (sized)              + priority / pressure
       │                                 │
       └── in-process asyncio             └── durable across restart
```

**EdgeQuake wins** durability.  
**LightRAG wins** stage isolation (parse failures don't block extract queue).

---

## Multi-Tenancy

| Concern | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| Workspace namespace | ✓ | ✓ UUID |
| Tenant isolation | △ | ✓ middleware |
| Per-workspace LLM config | △ addon_params | ✓ workspace service |
| Storage key prefixing | ✓ | ✓ workspace_hash_key |
| Cross-tenant leak tests | ✓ | ✓ |

Both adequate. EdgeQuake more explicit in API layer.

---

## Failure Recovery Flow

```text
  EdgeQuake startup (main.rs)
  ───────────────────────────

  1. migrate DB
  2. recover_orphaned_tasks()
  3. recover_orphaned_documents()
  4. start WorkerPool
  5. bind /health

  LightRAG startup
  ────────────────

  1. initialize_storages()
  2. pipeline processes in-flight docs
  3. (no durable task table by default)
```

---

## System Engineering Grade

| System | Grade | Notes |
|--------|:-----:|-------|
| LightRAG (default) | **C+** | Dev-friendly, fragile prod |
| LightRAG (Postgres prod) | **B+** | Requires expert setup |
| EdgeQuake | **A+** | Makefile, health, recovery, saga |

**EdgeQuake is the better system engineering artifact.** LightRAG is the better **research platform**.

---

## Scale-Out Honesty

Both are **single-writer ingestion** architectures today.

| Scale concern | LightRAG | EdgeQuake |
|---------------|:--------:|:---------:|
| Horizontal API | △ | △ |
| Sharded workers | ✗ | ✗ |
| Read replicas | △ PG | △ PG |
| Queue externalization | ✗ | ✗ (PG tasks) |

Neither is ready for **1000-doc/min ingest** without architectural work.
