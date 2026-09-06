# 13 — Honest Assessment (SPEC-146)

Verified against EdgeQuake code on 2026-09-05 (product pin v0.26.5). Spec-only; no product code yet.

## Verdict

The SPEC-146 pack is **directionally correct**: workspace walls are insufficient for mixed-classification GraphRAG; typed ANN ignores document allow-lists; hops amplify shared-entity leakage; caches/MCP need principal binding. Several early claims were **too neat** relative to the real list SSOT, principal identity, and PDP layering. This document locks what stays, what was wrong, and what we will not claim.

## What is solid (keep)

| Claim | Code proof |
|-------|------------|
| Typed ANN default ignores `document_ids` | [`vector_backend.rs`](../../edgequake/crates/edgequake-storage/src/vector_backend.rs) default `TypedEmbeddings`; [`try_typed_chunk_query`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/typed_read.rs) workspace-only; fleet `search` workspace-only |
| `document_filter` empty = all-pass; explicit IDs trusted | [`document_filter_resolver.rs`](../../edgequake/crates/edgequake-api/src/handlers/query/document_filter_resolver.rs) |
| `Permission` unused at HTTP | zero `require_permission` in `edgequake-api` |
| WebUI role drift | `user-management-card.tsx` `admin/developer/viewer` vs API `admin/user/readonly` |
| Migrations start at 150+ | highest landed = **149** |
| Cedar viable | `cedar-policy` 4.12 Apache-2.0, MSRV 1.89; workspace rustc 1.95 |
| Keyword cache unbound | [`llm_extractor.rs`](../../edgequake/crates/edgequake-query/src/keywords/llm_extractor.rs) hash(query, mode, model, lang) |
| No `/members` API | OpenAPI / routes |

## What was wrong or incomplete (corrected)

### 1. List SSOT is KV — RLS does not hide titles

[`list_documents`](../../edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs) reads via `document_metadata_scan` (KV + staging merge). Postgres RLS on `documents` is **not** the list PEP.

→ **LAW-146-17**. Finding F-146-33.

### 2. `status_counts` side channel

Same handler keeps counts **global** (SPEC-084 / GH-319). Unauthorized corpus size leaks through badges.

→ Override when ABAC on: counts over authorized set only. F-146-34 / EC-146-33.

### 3. Dual evaluators are a DRY trap

“Cedar **or** compiled SQL with semantic equivalence tests” forks the PDP. Operators and CI will drift.

→ **LAW-146-18**: SQL/set algebra for `workspace|acl|owner_only`; Cedar **only** for `classified`.

### 4. Master key is not a UUID

Auth stamps `user_id: "master-api-key"`. UUID-only `document_acl.principal_id` cannot represent Master/Worker.

→ **LAW-146-19**: tagged `PrincipalId`.

### 5. Reuse `edgequake-audit`

Crate already exists (`Authorization`, `DocumentQuery`, …). Do not invent a second audit logger. Extend `Permission::AuditLogRead` (already in enum) — do not add duplicate `AuditRead`.

### 6. Parse jobs: IDOR, not “no auth”

`/parse/jobs/{id}` is **not** in `is_public_request`. With auth on, any valid token can fetch any in-memory job UUID.

→ Correct F-146-20.

### 7. Context cache nuance

[`query_result_cache`](../../edgequake/crates/edgequake-query/src/cache/query_result_cache.rs) already hashes `allowed_document_ids`. Leak is when that field is **`None`** (today’s default path). Keyword/answer remain unbound.

### 8. Labels must dual-write

Admit writes SQL shell + KV `{uuid}-metadata`. SQL-only security columns leave list unlabeled.

→ EC-146-34: mint on both SSOTs (or list reads SQL).

### 9. ABAC + auth-off

`EDGEQUAKE_DOC_ABAC=1` with DEV_MODE/auth-off is unsafe. Refuse start.

→ **LAW-146-20**.

## What is hard (honest)

| Hard part | Why |
|-----------|-----|
| KV list PEP | Every list/search/autocomplete path must filter; miss one = title leak |
| Typed ANN + `ANY(allow)` | HNSW underfill when \|A\|/\|W\| small; measure before denorm |
| Provenance hops | Extract currently denormalizes multi-doc text onto hubs |
| Principal tagging | Master/worker/API key ≠ `users.user_id` |
| M1 scope | Schema+PDP+list+members+upload is too fat → M1a/M1b |
| Acc impact | Authorized-only hub text may drop Mix recall — **UNCONFIRMED** |

## Acc deferral (F-146-32) — M5 cut

**SPEC-001 Acc under ABAC is deferred for this cut.** We do not invent Acc, ΔAcc, or latency numbers.

- **Quality gate instead:** G-146-52 unauthorized-context harness **TARGET 0** (unauthorized chunk / `SECRET_TOKEN` never reaches LLM context after allow-set ∩ / context filter).
- **Why defer:** Authorized-only hub text and provenance-gated hops may change Mix recall; measuring without a controlled SPEC-001 ABAC slice would produce fiction.
- **Follow-up:** Run SPEC-001 medical-mid (or agreed slice) with `EDGEQUAKE_DOC_ABAC=1` after M4/M5 PEPs are stable; publish measured Acc then.

See also [10-acceptance.md](10-acceptance.md) § Acc measurement deferral.

## What we will not claim

```ascii
  ✗ “RLS alone enforces document ABAC on the Documents page”
  ✗ “Cedar evaluates every document on every list”
  ✗ “Post-filter is enough for typed ANN”
  ✗ “Acc improves / stays flat because of ABAC”  (deferred this cut — F-146-32)
  ✗ “Master key is a normal UUID principal”
  ✗ “Parse jobs are anonymous”  (they are authenticated IDOR)
  ✗ “ABAC GA” before M4 DoD + harness TARGET 0
```

## First-principles check

1. **Single authority** — one `AllowSetProvider`; no handler-local ACL.  
2. **Enforce at the real SSOT** — KV for list, SQL+JOIN for ANN, provenance for hops.  
3. **Fail closed** — missing allow-set when ABAC on is a bug, not workspace-all.  
4. **Evidence** — every correction maps to F-/EC-/G- IDs and a gate.  
5. **DRY** — one principal type, one audit crate, one SecurityFields component.

## Cross-refs

- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Laws → [00-first-principles.md](00-first-principles.md)  
- Findings → [01-finding-register.md](01-finding-register.md)  
- Code → [03-code-as-is.md](03-code-as-is.md)  
