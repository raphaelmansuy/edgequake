# 04 — Test protocol

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Shapes: [05](05-workloads.md)

Executable protocol. Pass/fail = **report card complete and internally consistent**,
not “hit an invented tokens/sec target” (LAW-153-8 / SPEC-063 proof ladder).

## 0. Target card (demo)

| Field | Value |
|-------|-------|
| Base URL | `https://demo.edgequake.com` |
| Tenant | `00000000-0000-0000-0000-000000000002` |
| Workspace | `00000000-0000-0000-0000-000000000003` |
| Headers | `Authorization: Bearer <token>` · `X-Tenant-ID: <tenant>` · workspace header/body as API requires |
| MCP | `https://demo.edgequake.com/mcp` · scopes `edgequake:read`, `edgequake:query` |
| Default layers | **L0 + L7 + L8** (Provider) |
| Ingest L1–L6 Provider | **Opt-in** dedicated tenant / off-window only |
| System L1–L6 | Local clocked mock (always allowed) |

Preflight (must all pass before a sweep):

```bash
curl -sf https://demo.edgequake.com/live
curl -sf https://demo.edgequake.com/ready
curl -sf https://demo.edgequake.com/health | tee measurements/preflight-health.json
# Assert: ready==true, schema.migration_required==false, task_queue.pressure in {normal,warn}
# Record: version, providers.llm, providers.embedding, schema.migrations_applied
```

Auth preflight: one authenticated `GET` workspace/docs or MCP `eq_workspace_list` must
return the pinned workspace (today: `…0003`). **401/403 → abort** (do not count as capacity).

## 1. Env card (pin everything)

Record in `measurements/<run_id>/env.json`:

```text
  base_url, tenant_id, workspace_id
  product_version, git/build if known
  llm_provider, llm_model
  embed_provider, embed_model, embed_dim
  EDGEQUAKE_LLM_CACHE / KEYWORD / QUERY_ANSWER  (cold vs warm)
  EDGEQUAKE_PROMPT_CACHE (+ TTL)
  EDGEQUAKE_TASK_MAX_WORKERS
  EDGEQUAKE_PROVIDER_MAX_INFLIGHT
  EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS
  EDGEQUAKE_DB_POOL_SIZE_{QUERY,INGEST,QUEUE,ADMIN}
  EDGEQUAKE_MAX_EXTRACTION_{ENTITIES,RECORDS}     # SPEC-117
  shape_id, bench_mode (system|provider), cache_mode (cold|warm)
  load_model (closed_loop|open_loop)
```

Change **one** field per comparative run (LAW-153-7).

## 2. Functional oracle (not Acc)

A unit is **oracle_ok** iff:

1. HTTP/MCP status indicates success (2xx / tool ok).
2. Payload parses to the expected schema (query answer or search hits non-null structure).
3. No timeout / cancel from the client harness.

Oracle does **not** score answer quality (that is SPEC-001 Acc). A fast wrong Acc answer
can still be capacity-good; a timeout is capacity-bad.

**Error classes** (excluded from goodput numerator; counted in USE errors):

| Class | Handling |
|-------|----------|
| `auth` | Abort run |
| `rate_limit` | Count as saturation signal at L0; not LLM knee |
| `timeout` | sojourn fail |
| `5xx` | error |
| `parse` | harness/product defect — investigate, do not hide |

## 3. SLO hypotheses (label as hypotheses until proven)

Default **starting hypotheses** for demo query (tune after first System cards):

| Layer | Metric | Hypothesis (demo query-chat) |
|-------|--------|------------------------------|
| L0 | reject rate | < 1% of offered (excluding intentional 429 probes) |
| L7 | `retrieval_time_ms` p90 | hypothesis — record only until proven |
| L8 | `ttft_ms` p90 | hypothesis |
| L8 | e2e `total_time_ms` p90 | hypothesis |
| L8 | oracle_ok | ≥ 99% in steady window |

Attainment for a unit: **all** selected SLOs true (AIPerf-style conjunction).  
Unfilled numeric thresholds must be marked `"status":"hypothesis"` in the report.

## 4. Load models

### Closed-loop (default for L7/L8 demo)

```text
  concurrency C in {1, 2, 4, 8, 16, ...}
  hold C until steady window completes
  stop when attainment < 0.90 OR error_guard OR saturation_guard
  knee = last C with attainment >= 0.90
```

### Open-loop (optional query)

```text
  offered arrival rate λ (Poisson or fixed interval)
  MLPerf-server style: score only latency-compliant completions
  knee = max λ with attainment >= 0.90
```

### Guards (stop early)

| Guard | Example signal |
|-------|----------------|
| Error | >5% 5xx/timeout in window |
| Saturation | task_queue pressure critical; pool acquire timeouts; provider 429 storm |
| Auth | any 401/403 mid-run |

## 5. Windowing

```text
  warmup:   discard first W seconds or first N completions (record W,N)
  steady:   fixed T seconds OR fixed M completions (prefer T>=60s for Provider)
  repeats:  3 independent runs; report median knee + min/max
```

## 6. Procedure — Provider demo (L7/L8)

1. Preflight §0.  
2. Cold cache mode (LAW-153-6).  
3. Shape = `query-chat-v1` ([05](05-workloads.md)) against pinned tenant/workspace.  
4. Closed-loop concurrency sweep §4.  
5. For each unit, write ledger fields ([03](03-token-ledger.md)).  
6. Compute attainment; find knee; record USE snapshot (`/health` operational + client errors).  
7. Optional second card: warm cache mode.  
8. Emit artifact §8.

MCP path (read-only): `eq_search` → `eq_fetch` / `eq_retrieve` may be used for L7-shaped
retrieve-only studies. Full L8 answer generation requires REST chat/query with credentials
that allow completion (MCP scopes are query/read — confirm whether answer tools exist for
your token; if not, REST is mandatory for L8).

## 7. Procedure — System bench (L1–L6)

1. Local EdgeQuake + Postgres; clocked mock LLM/embed (`service = overhead + slope×tokens`).  
2. Shape from [05](05-workloads.md) (`ingest-short-v1`, `ingest-dense-v1`, `queue-only-v1`).  
3. Same sweep/attainment math.  
4. Pair with Provider demo L7/L8 when attributing end-to-end wall time.

## 8. Artifact schema

Directory: `specs/153-workload-benchmark/measurements/<run_id>/`

```text
  env.json                 # §1
  preflight-health.json
  units.jsonl              # one JSON object per completed unit
  summary.json             # knee, attainment curve, USE
  SUMMARY.md               # human one-pager
```

`summary.json` minimum:

```json
{
  "spec": "153",
  "base_url": "https://demo.edgequake.com",
  "tenant_id": "00000000-0000-0000-0000-000000000002",
  "workspace_id": "00000000-0000-0000-0000-000000000003",
  "bench_mode": "provider",
  "cache_mode": "cold",
  "shape_id": "query-chat-v1",
  "layers": ["L0", "L7", "L8"],
  "load_model": "closed_loop",
  "knee": { "concurrency": null, "attainment": null, "status": "not_run" },
  "species_rates": {
    "llm_in_per_s": null,
    "llm_out_per_s": null,
    "embed_est_per_s": null
  },
  "slo": { "status": "hypothesis" },
  "gaps": ["G-153-VISION", "G-153-EMBED-USAGE"]
}
```

## 9. Pass / fail of a protocol run

| Result | Meaning |
|--------|---------|
| **PASS** | Env card complete; tenant/workspace echoed; units.jsonl non-empty; knee or explicit stop-guard recorded; species not illegally summed; gaps listed |
| **FAIL** | Missing pin, mixed cold/warm, mixed species sum, Acc used as oracle, or health LLM flag treated as capacity |
| **BLOCKED** | Demo not ready / auth failure / migration_required |

## 10. Safety on shared demo

- Prefer read/query load.  
- Cap concurrency (start ≤8) until a proven floor exists.  
- Do not delete documents or wipe workspace.  
- Do not run Vision re-OCR storms on shared PDFs.  
- Announce Provider ingest benches in the run `SUMMARY.md` if ever approved.

## 11. Higher concurrent-user extension

For closed-loop C→32, open-loop arrival rates, FAQ warm storms, **machine inventory**,
and charted business PDF:

→ **[07-highload-protocol.md](07-highload-protocol.md)**
