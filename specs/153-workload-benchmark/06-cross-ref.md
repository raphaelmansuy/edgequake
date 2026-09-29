# 06 — Cross-reference matrix

Parent: [README](README.md)

## Law ↔ layer ↔ metric ↔ code

| Law | Layer(s) | Metric / rule | Code / external |
|-----|----------|---------------|-----------------|
| LAW-153-1 | L4 L5 L8 | Typed species; no mixed sums | `CostBreakdownStats`; `estimate_embed_tokens`; AIPerf species split |
| LAW-153-2 | all | Goodput @ knee ≥90% attainment | DistServe goodput; AIPerf `--goodput` |
| LAW-153-3 | L1 L7 L8 | service vs sojourn; Little’s law | queue wait + `QueryStats` times |
| LAW-153-4 | all | System + Provider dual cards | mock clocked provider; live openai on demo |
| LAW-153-5 | all | Frozen `shape_id` | [05-workloads](05-workloads.md) |
| LAW-153-6 | L7 L8 | Cache mode cold\|warm | SPEC-103 / SPEC-126 flags |
| LAW-153-7 | all | One variable; pin tenant | demo `…0002` / `…0003` |
| LAW-153-8 | all | Steady state, p90/p99, USE, proof ladder | SPEC-063 ladder; `/health` operational |

## Layer ↔ knobs ↔ sibling specs

| Layer | Primary knobs | Sibling |
|-------|---------------|---------|
| L0 | rate limiter 100/20 | — |
| L1 | `EDGEQUAKE_TASK_MAX_WORKERS`, `EDGEQUAKE_PROVIDER_MAX_INFLIGHT` | SPEC-090 claim |
| L2 | Vision vs EdgeParse | SPEC-134 / 151; gap G-153-VISION |
| L3 | chunk size / token estimator | SPEC-135 |
| L4 | `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS`, SPEC-117 caps | SPEC-117 |
| L5 | `EDGEQUAKE_EMBEDDING_BATCH_SIZE` | SPEC-016 capacity model |
| L6 | `EDGEQUAKE_DB_POOL_SIZE_INGEST` | SPEC-090, SPEC-112 |
| L7 | `EDGEQUAKE_DB_POOL_SIZE_QUERY`, ANN GUCs | SPEC-063, SPEC-090 |
| L8 | stream TTFT, max tokens, reasoning effort | SPEC-109, SPEC-001 (quality only) |

## Sibling specs — boundary

| Spec | Owns | Hands off to SPEC-153 |
|------|------|------------------------|
| [SPEC-001](../001-benchmark/) | Acc / fair quality | Token goodput |
| [SPEC-016 §006](../016-datalayer-audit/006-capacity/) | HNSW RAM + write RT model | Live token knee |
| [SPEC-063](../063-architecture-capacity-assessment/) | Hard caps / physics / proven ANN floors | LLM-token layers |
| [SPEC-090](../090-performance/) | DB counter/pool/claim/ANN order | End-to-end RAG tokens |
| [SPEC-103](../103-llm-cache/) | Keyword/answer cache semantics | Cache as bench **mode** |
| [SPEC-112](../112-connection-pool/) | Role pools | Pool USE at knee |
| [SPEC-117](../117-extraction-budget/) | Extract quantity caps | Caps as shape variable |
| [SPEC-126](../126-provider-kv-cache/) | Provider prompt cache | Warm mode fields |
| [SPEC-152](../152-new-mcp-contract/) | MCP tool contract | Demo MCP as L7 probe surface |

## Demo pin cross-ref

| Item | Value | Where enforced |
|------|-------|----------------|
| Base URL | `https://demo.edgequake.com` | [00](00-why.md), [04](04-protocol.md) |
| Tenant | `00000000-0000-0000-0000-000000000002` | env card; every units.jsonl row |
| Workspace | `00000000-0000-0000-0000-000000000003` | MCP `eq_workspace_list`; REST headers |
| MCP resource | `https://demo.edgequake.com/mcp` | OAuth protected resource metadata |
| Scopes | `edgequake:read`, `edgequake:query` | Limits Provider ingest on MCP |
| LLM | openai `gpt-5.4-mini` | `/health` providers (re-check each run) |
| Embed | `text-embedding-3-small` @ 1536 | `/health` providers |
| Default shape | `query-chat-v1` | [05](05-workloads.md) |

Legacy UUID pair also appears in product tests as default tenant/workspace
(e.g. `e2e_document_deletion_postgres.rs`) — the demo pin matches that convention.

## Gaps ↔ blockers

| Gap | Blocks claim | Unblock |
|-----|--------------|---------|
| G-153-VISION | L2 Vision tokens/sec | Instrument parse metrics |
| G-153-EMBED-USAGE | True embed goodput | Plumb provider embedding usage |
| G-153-HEALTH-LLM | Health-based LLM readiness | Live probe or drop from USE |
| G-153-OLLAMA-SHIM | Local compat “usage” | Prefer real usage fields |

## Document map

| Doc | Role |
|-----|------|
| [README](README.md) | Entry, non-goals |
| [00-why](00-why.md) | 5-WHY + demo facts |
| [01-first-principles](01-first-principles.md) | LAW-153 |
| [02-service-layers](02-service-layers.md) | L0–L8 |
| [03-token-ledger](03-token-ledger.md) | Species rules |
| [04-protocol](04-protocol.md) | Executable protocol |
| [05-workloads](05-workloads.md) | Frozen shapes |
| [06-cross-ref](06-cross-ref.md) | This matrix |
| [measurements/](measurements/) | Run artifacts |
