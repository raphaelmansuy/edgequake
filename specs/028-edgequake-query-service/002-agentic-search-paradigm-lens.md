# 002 — Agentic Search Paradigm Lens

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [001-first-principles-five-whys.md](./001-first-principles-five-whys.md) | [024-004](../024-egdequake-audit/004-lightrag-expert-lens.md) | [024-005](../024-egdequake-audit/005-graphrag-expert-lens.md)  
**Research baseline:** June 2026 production patterns

---

## Definitions (Operational)

### Pipeline RAG (Classic RAG)

One retrieval pass → one generation pass. Optimized for **latency and cost** on factoid Q&A.

```
  Query ──► Embed ──► Vector Top-K ──► LLM ──► Answer
```

EdgeQuake default user flow (`POST /query` without flags) is Pipeline RAG with LightRAG-class retrieval.

---

### LightRAG (EdgeQuake's Retrieval Model)

Dual-level retrieval over a **flat knowledge graph** + vector indexes:

| Level | Vector target | Purpose |
|-------|---------------|---------|
| **Low (Local)** | Entity embeddings | Precise entity-neighborhood answers |
| **High (Global)** | Relationship embeddings | Thematic / relational answers |
| **Naive** | Chunk embeddings | Direct passage retrieval |

EdgeQuake implements LightRAG semantics via `Local`, `Global`, `Hybrid`, `Mix`, and `Naive` modes ([024-003](../024-egdequake-audit/003-query-retrieval-audit.md)). Incremental graph updates supported via ingestion pipeline.

**What LightRAG is NOT in EdgeQuake:** GraphRAG-style hierarchical community summaries precomputed at index time.

---

### GraphRAG (Microsoft-style)

Indexing builds entity graph → **community detection** → **hierarchical summaries** per community. Query routes to community level for corpus-wide "thematic" questions.

| Capability | GraphRAG | EdgeQuake (LightRAG-class) |
|------------|----------|----------------------------|
| Entity + relation extraction | ✅ | ✅ |
| Multi-hop via graph traversal | ✅ (community + local) | ✅ (local/global modes) |
| Precomputed community reports | ✅ | ❌ |
| Global "themes across corpus" | ✅ Strong | ⚠️ Moderate (Global mode, no summaries) |
| Index cost | High ($6–7/book cited) | Lower (incremental) |
| Incremental update | Expensive rebuild | ✅ Supported |

**Complementarity:** GraphRAG wins on **corpus-wide analytical questions**. EdgeQuake wins on **dynamic corpora, incremental ingestion, and dual-level hybrid retrieval** at lower cost.

---

### Agentic Search / Agentic RAG

**Not a retrieval algorithm** — an **orchestration paradigm** where an LLM agent controls retrieval as a tool ([BestHub 2026](https://www.besthub.dev/articles/multi-hop-reasoning-vs-document-parsing-comparing-graphrag-lightrag-agenticrag-and-ragflow-3e05cadc0394)):

```
  ┌──────────────────────────────────────────────────────────────────┐
  │                     Agentic Search Loop                          │
  │                                                                  │
  │   ┌─────────┐    ┌──────────────┐    ┌─────────────────────┐   │
  │   │  Plan   │───►│  Retrieve    │───►│  Evaluate context   │   │
  │   │  query  │    │  (tool call) │    │  sufficient?        │   │
  │   └─────────┘    └──────────────┘    └──────────┬──────────┘   │
  │        ▲                                         │              │
  │        │              NO                        │ YES          │
  │        └──────── rewrite / refine query ──────────┘              │
  │                                         │                        │
  │                                         ▼                        │
  │                              ┌─────────────────────┐             │
  │                              │  Synthesize answer  │             │
  │                              │  (may use other     │             │
  │                              │   tools too)        │             │
  │                              └─────────────────────┘             │
  └──────────────────────────────────────────────────────────────────┘
```

**Key behaviors:**
1. **Decide WHEN to retrieve** (not every turn)
2. **Decide WHAT to retrieve** (mode, filters, sub-queries)
3. **Evaluate quality** (coverage, contradictions, gaps)
4. **Iterate** until evidence sufficient or budget exhausted
5. **Compose tools** — vector, graph, SQL, web, **EdgeQuake context**

Production implementations: LangGraph state machines, LlamaIndex agent workflows, OpenAI deep research with MCP tools.

---

## The Complementarity Matrix (June 2026)

```
                    RETRIEVAL MECHANISM
          ┌─────────────┬─────────────┬─────────────┐
          │  Classic    │  LightRAG   │  GraphRAG   │
          │  Vector RAG │  (EdgeQuake)│  (MSFT)     │
  ┌───────┼─────────────┼─────────────┼─────────────┤
  │ Simple│  ★★★★★     │  ★★★★       │  ★★         │
  │ Q&A   │  fast/cheap │  + relations│  overkill   │
  ├───────┼─────────────┼─────────────┼─────────────┤
  │ Multi-│  ★★         │  ★★★★★     │  ★★★★★      │
  │ hop   │  misses     │  graph prov │  community  │
  ├───────┼─────────────┼─────────────┼─────────────┤
  │ Corpus│  ★★         │  ★★★       │  ★★★★★      │
  │ themes│  no global  │  rel vectors│  summaries  │
  ├───────┼─────────────┼─────────────┼─────────────┤
  │Dynamic│  ★★★★★     │  ★★★★★     │  ★★         │
  │ corpus│  re-embed   │  incremental│  re-cluster │
  └───────┴─────────────┴─────────────┴─────────────┘

          ORCHESTRATION LAYER (Agentic RAG)
          ┌─────────────────────────────────────┐
          │ Routes query → picks tool(s)      │
          │ EdgeQuake = "relational retrieval"  │
          │ tool in the agent toolbox           │
          └─────────────────────────────────────┘
```

**Production hybrid (2026 consensus):**
- Fast path: Classic/Naive for simple lookups
- Relational path: LightRAG/EdgeQuake for entity-heavy questions
- Analytical path: GraphRAG when community summaries justify cost
- **Agent layer:** routes between them; EdgeQuake exposed as **`search` tool**

---

## What EdgeQuake Provides vs What the Agent Provides

| Responsibility | EdgeQuake Query Context Service | External / Native Agent |
|----------------|--------------------------------|-------------------------|
| Embedding + vector search | ✅ | — |
| Graph entity/rel retrieval | ✅ | — |
| Chunk provenance + lineage | ✅ | — |
| Mode selection (Mix/Local/…) | ✅ (default Mix) | Can override per call |
| Document scoping | ✅ | Passes filter |
| Loop control (iterate?) | — | ✅ |
| Sub-query decomposition | — | ✅ |
| Answer synthesis | Optional (`/query`) | ✅ (preferred for agents) |
| Tool budget / step limits | — | ✅ |
| Cross-source reasoning | — | ✅ |

EdgeQuake is the **retrieval substrate**; Agentic Search is the **control plane**.

---

## Agentic Search Requirements on the Retrieval API

Derived from LangGraph / MCP / OpenAI deep research patterns:

| Requirement | Why | EdgeQuake response |
|-------------|-----|-------------------|
| **Structured graph** | Agent traverses relationships | `subgraph.entities` + `subgraph.relationships` |
| **Full chunk text** | Agent quotes evidence | `chunks[].content` at `agent` granularity |
| **Stable IDs** | Multi-step fetch | `chunk_id`, `entity_name`, `relationship_id` |
| **Search + Fetch split** | MCP pattern | `edgequake_search` → IDs; `edgequake_fetch` → bundle |
| **Provenance** | Citations, audit | `lineage` block per item |
| **Coverage signal** | Stop or iterate decision | `retrieval_quality.coverage_score` |
| **Truncation honesty** | Agent knows gaps | `truncation.dropped`, `is_truncated` |
| **Mode transparency** | Debug routing | `mode`, `mode_arms` breakdown |
| **Deterministic replay** | Eval / regression | `retrieval_fingerprint` hash |
| **Stateless** | MCP 2026-07-28 | No server session; pass workspace in args |

---

## Native Agentic Search Mode (Future — Phase 6)

EdgeQuake will eventually host an **in-process agent loop** that calls `QueryContextService` internally:

```
  User Query
      │
      ▼
  ┌─────────────────────┐
  │ EdgeQuake Agent     │  (new: edgequake-agent crate)
  │ Controller          │
  └─────────┬───────────┘
            │ tools: retrieve_context, fetch_document, graph_neighbors
            ▼
  ┌─────────────────────┐
  │ QueryContextService │  ◄── same SSOT as HTTP + MCP
  └─────────────────────┘
```

SPEC-028 **does not implement** the agent loop — it **enables** it by delivering the tool contract.

---

## Anti-Patterns (Do Not Build)

| Anti-pattern | Why wrong |
|--------------|-----------|
| Agent calls `/query` and ignores `answer` | Wastes LLM tokens; wrong SLA |
| Return only flat `sources[]` for agents | Destroys graph; forces reconstruction |
| Hide mode selection from agent | Agent cannot specialize sub-queries |
| Server-side agent session state | Breaks MCP 2026 stateless model |
| Reimplement retrieval in MCP layer | Violates FP-028-02; drift from engine |

---

## Research References

- [Pipeline vs Agentic vs KG RAG (Medium, 2026)](https://medium.com/@Micheal-Lanham/pipeline-rag-vs-agentic-rag-vs-knowledge-graph-rag-what-actually-works-and-when-47a26649a457)
- [LightRAG vs GraphRAG vs AgenticRAG comparison (BestHub)](https://www.besthub.dev/articles/multi-hop-reasoning-vs-document-parsing-comparing-graphrag-lightrag-agenticrag-and-ragflow-3e05cadc0394)
- [Graph RAG in Production 2026](https://www.paperclipped.de/en/blog/graph-rag-production/)
- [MCP 2026-07-28 RC — stateless tools](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [OpenAI MCP search/fetch pattern](https://developers.openai.com/api/docs/mcp)
