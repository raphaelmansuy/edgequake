# 001 — AI Engineering Lens

**Cross-ref:** [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [004 Query](../004-query/001-query-comparison.md)

---

## Prompt & Extraction Engineering

| Capability | LightRAG | EdgeQuake |
|------------|:--------:|:---------:|
| Entity extraction prompts | ✓ profiles | ✓ schema-driven |
| Gleaning prompts | ✓ | ✓ |
| Merge summarization | ✓ force/skip | ✓ |
| Keyword extraction | ✓ dual-level | ✓ |
| Response type control | ✓ | ✓ |
| Section context injection | ✓ breadcrumb | △ |
| Multimodal prompts | ✓ VLM | ✗ |
| Think-tag removal | ✓ | △ |

LightRAG `prompt.py` + `prompt_multimodal.py` — mature prompt library with profile validation.

EdgeQuake `edgequake-pipeline/src/prompts/` — functional but narrower.

---

## Retrieval Stack (AI Engineering View)

```text
  June 2026 "good RAG" stack          LightRAG    EdgeQuake
  ──────────────────────────          ────────    ─────────
  Dense retrieval                     ✓           ✓
  Sparse retrieval (BM25)             ✗           ✓
  Hybrid fusion (RRF)                 ✗           ✓
  Cross-encoder rerank                △           ✓
  Multi-hop graph                     △           ✓
  Query intent routing                ✗           ✓
  Conversation-aware retrieval        ✗           ✓
  Agentic re-retrieval                ✗           ✗
```

EdgeQuake implements **more of the 2024-2026 retrieval recipe** than stock LightRAG.

---

## LLM Provider Architecture

| Feature | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| Role-based LLM (extract/query/summary) | ✓ `llm_roles.py` | △ workspace config |
| Provider count | 10+ bindings | 4+ (OpenAI, Ollama, mock, LMStudio) |
| LLM response cache | ✓ hashing_kv | △ |
| Embedding batch | ✓ | ✓ |
| Hybrid LLM+embed providers | △ | ✓ SPEC-033 |
| VLM for PDF/images | ✓ | ✓ PDF vision only |

LightRAG `llm_roles.py` separates extract/query/summary with priority queues — sophisticated.

EdgeQuake workspace-level provider selection is **cleaner for multi-tenant** but less granular on role separation.

---

## Quality Risks

1. **EdgeQuake Mix default** — best quality, highest cost. Intent routing mitigates but adds LLM classification dependency.
2. **LightRAG no BM25** — misses exact-match retrieval on codes, IDs, names.
3. **Both** — no faithfulness verification before returning answers.
4. **EdgeQuake** — community_id expansion can inject off-topic entities if Louvain clusters are loose.

---

## AI Engineering Grade

| System | Grade | Rationale |
|--------|:-----:|-----------|
| LightRAG | **B** | Solid extraction; retrieval stack dated |
| EdgeQuake | **B+** | Modern fusion + rerank; no agentic layer |

**Recommendation:** Port LightRAG's **prompt profiles + section context** into EdgeQuake before adding more retrieval modes.
