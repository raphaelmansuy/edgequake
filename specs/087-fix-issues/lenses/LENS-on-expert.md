# LENS — O(n) Expert

> **Laws**: LAW-30, LAW-31, LAW-33  
> **Findings**: `iss087_stats_n1`, `iss087_anon_mint`, `iss087_kv_count_trait`

---

## 1. Question

What are the asymptotic costs today, and what budgets must the fix meet?

---

## 2. Stats path cost model (#334)

Let `D` = documents in workspace, `C_d` = chunks per doc, `B` = average chunk JSON bytes (may include large fields / historical embeddings).

| Step (current) | Cost |
|----------------|------|
| `keys_with_prefix` per doc | Θ(D) round-trips |
| `get_by_ids` per doc with chunks | Θ(D) round-trips + Θ(Σ C_d · B) bytes |
| JSON parse + `embedding` check | Θ(Σ C_d) CPU |

Against `STATS_FETCH_TIMEOUT = 4s`:

- Even at 1 ms/RTT, `D ≈ 9000` ⇒ ~9s of prefix listings alone → hard timeout.  
- Payload fetch makes it worse (reporter: 136s+ before timeout culture).  
- Existing tests stress ~50 docs → **false green**.

### Target complexity

| Path | Target |
|------|--------|
| PG product | O(1) round-trips for embedding/chunk aggregates (indexed COUNT) |
| Trait default (non-PG) | May remain O(D); document as fallback only |
| Cache | 60s TTL + stale-if-error remains; must not be the “fix” |

### Budget pin

Cold-cache stats for D ≥ 500 (e2e) and D ≥ 5000 (manual/prod) must complete in &lt; 4s wall clock on a healthy local PG.

---

## 3. Identity growth (#335)

| Model | `users` rows |
|-------|----------------|
| Current (auth off) | Θ(browsers × sessions that chat) — unbounded |
| Shared guest | Θ(tenants) for guest rows — bounded |
| Auth on + JWT bind | Θ(real accounts) — intended |

Admin list without filter is O(users) UI cost and cognitive load — filter reduces perceived n.

---

## 4. Anti-patterns

- Raising timeout instead of reducing complexity  
- Parallelizing the N+1 loop (still Θ(D) work; hides under load)  
- Counting by downloading vectors  
- Minting users “just in case” on read endpoints

---

## 5. Acceptance for this lens

- [ ] No Θ(D) sequential KV payload fetch on stats hot path  
- [ ] Guest identity growth O(1) per tenant when auth off  
- [ ] Scale e2e `iss087_e_scale_stats` green
