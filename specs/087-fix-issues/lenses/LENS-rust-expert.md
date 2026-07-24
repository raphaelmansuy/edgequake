# LENS — Rust Expert System

> **Laws**: LAW-31, LAW-32, LAW-33  
> **Findings**: `iss087_kv_count_trait`, `iss087_stats_n1`, `iss087_anon_mint`

---

## 1. Question

How do we implement the fixes with idiomatic Rust, trait defaults, and DRY helpers without forking behavior behind `cfg`?

---

## 2. Stats — trait design (#334)

Trait name in tree: **`KVStorage`** (not `KvStore`).

```rust
/// Count chunk entries relevant to embedding metrics for the given document ids.
/// Default: per-document prefix scan (correctness fallback).
/// Postgres: single aggregate; prefer chunk-key / relational SSOT over jsonb embedding field.
async fn count_embedded_chunks_for_docs(&self, doc_ids: &[String]) -> Result<usize> {
    // default loop — same shape as today's stats.rs body
}
```

| Rule | Detail |
|------|--------|
| Default method | Enables non-PG adapters without boilerplate (LAW-33) |
| Postgres override | Use `self.table_name` / qualified name; bind `doc_ids`; short-circuit empty |
| Handler | One call; map errors to `ApiError::Internal` consistently |
| Tests | Unit test default; PG integration asserts aggregate path / count |

Avoid duplicating the loop in `stats.rs` after the trait lands.

---

## 3. Identity — helper design (#335)

Today:

```text
ensure_postgres_user_exists → ensure_anonymous_user_in_postgres(client_uuid)
```

Target:

```text
ensure_postgres_user_exists(state, tenant, resolved_user)
  → if auth_enabled && real_user: ensure real row exists (or trust login bootstrap)
  → else if allow_anonymous: ensure_shared_guest(tenant)
  → else: Err(Unauthorized)
```

| Concern | Guidance |
|---------|----------|
| `#[cfg(feature = "postgres")]` | Keep no-op without feature — same as today |
| Deterministic guest UUID | `Uuid::new_v5(NAMESPACE, tenant_id.as_bytes())` or documented constant per tenant |
| Middleware | Bind JWT `sub` → `TenantContext.user_id` before handlers |
| Contract test | Stop pinning old function name; pin shared-guest behavior / symbol |

---

## 4. SOLID checklist

- **SRP**: stats handler does not contain SQL; identity helper owns mint policy  
- **OCP**: new count method; adapters override  
- **DIP**: HTTP depends on traits/helpers  
- **DRY**: three chat/conversation callers keep one bootstrap entrypoint  

---

## 5. Acceptance for this lens

- [ ] `count_embedded_chunks_for_docs` on `KVStorage` with Postgres override  
- [ ] No N+1 loop left in `stats.rs`  
- [ ] Single bootstrap entrypoint with guest/real/deny policy  
- [ ] `cargo clippy -p edgequake-api -p edgequake-storage` clean for touched code
