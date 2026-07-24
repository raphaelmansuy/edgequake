# SPEC-087 — Cross-Reference Matrix

> Finding ↔ lenses ↔ laws ↔ wave ↔ verification IDs

| Finding ID | Study | Primary lenses | Laws | Wave | Verify IDs |
|------------|-------|----------------|------|------|------------|
| `iss087_stats_n1` | [F-stats-n1-embedding](findings/F-stats-n1-embedding.md) | O(n), Full Stack, Rust | 31,33 | 1 | `iss087_v_stats_under_timeout`, `iss087_e_scale_stats` |
| `iss087_kv_count_trait` | [F-stats-n1-embedding](findings/F-stats-n1-embedding.md) | Rust, Postgres | 33 | 1 | `iss087_v_count_trait` |
| `iss087_embedding_ssot` | [F-stats-n1-embedding](findings/F-stats-n1-embedding.md) | Postgres, Full Stack | 32 | 1 | `iss087_v_embedding_ssot` |
| `iss087_anon_mint` | [F-anon-user-mint](findings/F-anon-user-mint.md) | Postgres, Product Owner, Full Stack | 29,30 | 2 | `iss087_v_shared_guest`, `iss087_e_incognito_no_growth` |
| `iss087_jwt_userid` | [F-anon-user-mint](findings/F-anon-user-mint.md) | Full Stack, Rust | 29,30 | 2 | `iss087_v_jwt_bind`, `iss087_e_auth_on_no_anon` |
| `iss087_admin_anon_filter` | [F-anon-user-mint](findings/F-anon-user-mint.md) | Design, Product Owner | 29 | 2 | `iss087_v_admin_filter` |
| `iss087_allow_anonymous_flag` | [F-anon-user-mint](findings/F-anon-user-mint.md) | Product Owner, Rust | 30 | 2 | `iss087_v_allow_anonymous_flag` |
| `iss087_anon_cleanup` | [F-anon-user-mint](findings/F-anon-user-mint.md) | Postgres, Product Owner | 29,30 | 3 | `iss087_v_cleanup_playbook` |

---

## Lens coverage

| Lens | File | Owns / challenges |
|------|------|-------------------|
| Postgres | [LENS-postgres.md](lenses/LENS-postgres.md) | FK, RLS, COUNT, table qualification |
| O(n) | [LENS-on-expert.md](lenses/LENS-on-expert.md) | 4s budget, unbounded users |
| Full Stack | [LENS-fullstack.md](lenses/LENS-fullstack.md) | FE header → bootstrap → admin; stats path |
| Product Owner | [LENS-product-owner.md](lenses/LENS-product-owner.md) | Trust, demo vs prod, severity |
| Rust Expert | [LENS-rust-expert.md](lenses/LENS-rust-expert.md) | Trait defaults, helper policy |
| Design | [LENS-design.md](lenses/LENS-design.md) | Guest badge, stale stats UX |

---

## Explicit non-dependencies / corrections

| Claim | Reality |
|-------|---------|
| #334 only needs trait default (call sites already merged) | **False** — loop + Postgres override also missing |
| Raising `STATS_FETCH_TIMEOUT` fixes #334 | **False** — LAW-31 |
| Filtering Admin UI alone fixes #335 | **False** — INSERTs must stop / bound |
| Nullable `conversations.user_id` required | **False** — shared guest preserves FK |
| `jsonb_exists(embedding)` is metric SSOT | **False** on current write path — LAW-32 |
| Product auth default is off | **False** — product default on; **dev/demo** often off |

---

## Dependency graph

```text
Wave 1 (stats)
  iss087_kv_count_trait ──► iss087_stats_n1
  iss087_embedding_ssot ──► iss087_stats_n1

Wave 2 (identity)
  iss087_anon_mint
       ├──► iss087_jwt_userid
       ├──► iss087_admin_anon_filter
       └──► iss087_allow_anonymous_flag

Wave 3
  iss087_anon_cleanup (after shared guest exists)
  e2e matrix + GitHub comments
```
