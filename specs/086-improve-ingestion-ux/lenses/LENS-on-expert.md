# LENS — O(n) Expert

**Job:** bound cost of staging visibility, polling, and WS fan-in so format parity does not regress latency.  
**Cites:** LAW-24, LAW-26 · `ux086_staging_list` · `ux086_sparse_md_events`

---

## 1. Cost centers

| Operation | Complexity (today) | Risk if naive |
|-----------|-------------------|---------------|
| `keys_with_prefix("staging:")` + get_by_ids | O(S) staging keys per progress/list call | Full scan on every documents poll |
| Documents list limited load | O(min(N, limit)) | Staging merge must respect limit/tenant filters |
| FE poll `/ingestion/{id}/progress` | O(1) per track; interval 2s/5s | Many concurrent uploads → O(U) polls |
| WS broadcast | O(subscribers × events) | Stage events every chunk → flood |
| Chunk metadata write every 3 chunks | O(C/3) | Small C → 0 updates (UX idle) |

N = workspace docs, S = staging docs, U = active upload tracks, C = chunks.

---

## 2. Design rules

1. **One staging merge implementation** — do not scan staging in each handler separately (DRY + fewer bugs).  
2. **Tenant filter before append** — staging values must match tenant/workspace (already in 068 helper).  
3. **Prefer indexed/wsdoc evolution later** — Wave 1 may scan staging prefix; document O(S) and add index if S grows (OCP later).  
4. **Stage WS = transition only** — emit on stage change, not per chunk (LAW-26). Keep every-3rd chunk progress for N/M granularity.  
5. **Poll backoff** — keep 5s when WS connected, 2s when not; never 200ms spam.  
6. **List limit** — staging in-flight docs should appear even when final list is capped (pin: include all staging for workspace OR reserve slots — choose in Wave 1: *include all tenant-matching staging metadata*, typically S ≪ N).

---

## 3. Recommended Wave 1 merge complexity

```text
list_inflight = load_final_limited(L) ∪ load_staging_filtered(tenant)
cost ≈ O(L + S) KV reads per list poll
```

Acceptable for interactive UI when S is small (admit concurrency ≪ L).

If S becomes large: add `wsdoc` staging index entry (skip today per `workspace_document_index` — revise carefully with promote).

---

## 4. Event budget

| Event | Cadence | Purpose |
|-------|---------|---------|
| Stage transition WS | O(stages) per doc ≈ ≤12 | Unstick small MD |
| ChunkProgress | every 3 chunks | N/M bar |
| PdfPageProgress | per page | PDF converting only |
| Poll | 2–5s | Fallback |

Forbidden: per-token or per-entity WS for progress UI.

---

## 5. Acceptance

| ID | Gate |
|----|------|
| ON-086-01 | Staging merge unit-tested; no unbounded full-KV scan of non-staging keys |
| ON-086-02 | Stage WS test with C &lt; 3 still advances (`ux086_e_small_md`) |
| ON-086-03 | Document O(L+S) in finding; follow-up index if needed |
