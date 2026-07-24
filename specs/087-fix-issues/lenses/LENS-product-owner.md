# LENS — Product Owner

> **Laws**: LAW-29, LAW-30, LAW-31  
> **Findings**: `iss087_anon_mint`, `iss087_admin_anon_filter`, `iss087_stats_n1`

---

## 1. Question

What does “done” mean for operator trust and dashboard reliability?

---

## 2. JTBD

| Actor | Job | Today | Done |
|-------|-----|-------|------|
| Self-host operator | Trust Users panel | Sees unknown `anon_*` accounts | Only real (or clearly labeled guest) identities |
| Demo user | Chat without login | Works, but pollutes DB | Still works; one shared guest |
| Security-conscious deploy | No implicit accounts | Unbounded inserts when auth off | Flag to refuse OR bounded guest |
| Analyst / admin | Open dashboard at scale | Spinner / 500 | Stats &lt; few seconds, accurate counts |

---

## 3. Severity

| Issue | Sev | Why |
|-------|-----|-----|
| #335 | P1 (P0 if public + auth off) | Trust + unbounded growth; not login RCE |
| #334 | P0 at scale | Product unusable (dashboard dead) |

Reporter #335 expected: “should not create any dummy users.” Locked compromise: **no per-browser dummies**; one explicit shared guest when auth off (preserves demo). Strict mode via `EDGEQUAKE_ALLOW_ANONYMOUS=false`.

---

## 4. Anti-goals

- Breaking open `make dev` chat without documenting the guest model  
- Hiding the bug by filtering UI only while INSERTs continue  
- Declaring #334 fixed if timeout stops but `embedding_count` stays wrong  
- Closing GitHub issues before e2e gates in [e2e/README.md](../e2e/README.md) pass

---

## 5. Messaging

- Guest accounts: label “Guest (system)” in Admin when shown.  
- Stats: keep existing stale banner (P-G13); after fix it should be rare.  
- Release notes: call out both as bugfixes with config note for `EDGEQUAKE_ALLOW_ANONYMOUS`.

---

## 6. Acceptance for this lens

- [ ] Operator-facing Users trust restored  
- [ ] Dashboard loads at reporter scale class  
- [ ] GitHub comments posted with root cause + SPEC-087 link (Wave 3)
