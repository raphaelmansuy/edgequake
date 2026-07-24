# SPEC-087 — Lenses

Multi-perspective studies. Each lens must cite laws from [00-first-principles.md](../00-first-principles.md) and findings from [01-finding-register.md](../01-finding-register.md).

| Lens | File | Primary question |
|------|------|------------------|
| Postgres Expert | [LENS-postgres.md](LENS-postgres.md) | FK, RLS, COUNT aggregates, table qualification |
| O(n) Expert | [LENS-on-expert.md](LENS-on-expert.md) | Can stats / identity stay inside latency & growth budgets? |
| Full Stack | [LENS-fullstack.md](LENS-fullstack.md) | Where does identity / stats break across FE→API→DB? |
| Product Owner | [LENS-product-owner.md](LENS-product-owner.md) | What does “done” mean for operator trust + dashboard? |
| Rust Expert System | [LENS-rust-expert.md](LENS-rust-expert.md) | Trait defaults, cfg gates, DRY helpers, SOLID seams |
| Design | [LENS-design.md](LENS-design.md) | Users panel + stats stale UX without clutter |

Inherit DRY/SOLID from [SPEC-017](../../017-dry-and-solid-audit/) and pack shape from [SPEC-086 lenses](../../086-improve-ingestion-ux/lenses/).
