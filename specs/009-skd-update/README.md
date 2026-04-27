# SPEC-009 — SDK parity with EdgeQuake API

**Branch:** `feat/sdk`  
**Authority:** `edgequake/crates/edgequake-api/src/routes.rs` and handler modules under `edgequake-api/src/handlers/`.  
**SDK roots:** `sdks/rust/`, `sdks/python/`, `sdks/typescript/` (Node-compatible; published as `@edgequake/sdk`).

## Documents in this folder

| Document | Purpose |
|----------|---------|
| [API-CANON.md](./API-CANON.md) | Canonical HTTP surface (method + path) derived from `routes.rs`. Single cross-reference index. |
| [RUST-SDK-GAP.md](./RUST-SDK-GAP.md) | Gap analysis: Rust SDK ↔ API. |
| [python-skd/GAP.md](./python-skd/GAP.md) | Gap analysis: Python SDK ↔ API. |
| [node-js/GAP.md](./node-js/GAP.md) | Gap analysis: TypeScript/Node SDK ↔ API. |
| [IMPLEMENTATION-PROOF.md](./IMPLEMENTATION-PROOF.md) | Executable proof: unit tests + `make sdk-e2e` / `sdk-e2e-with-stack`. |
| [SDK-QUALITY-ASSESSMENT.md](./SDK-QUALITY-ASSESSMENT.md) | Brutal honest tiering: Rust/Python/TS vs Go/C#/Java/Kotlin vs PHP/Ruby/Swift. |
| [SDK-PRE-ASSESSMENT.md](./SDK-PRE-ASSESSMENT.md) | Pre-flight methodology and coverage dimensions before each OODA batch. |
| [OODA-100-REGISTRY.md](./OODA-100-REGISTRY.md) | Numbered backlog (100 rows); git history is completion truth. |
| [OODA-LOG.md](./OODA-LOG.md) | Recorded Observe→Act cycles (phantom removal, path/body alignment). |

## First principles

1. **Code is law** — Routes and DTOs in the Axum crate override any prose in older SDK comments.
2. **DRY** — Tenant-scoped workspace listing/creation lives under **tenants**, not duplicated as ambiguous `workspaces` helpers unless the API does so.
3. **SOLID** — SDK mirrors API resource boundaries (`documents`, `graph`, `tenants`, `workspaces`, `admin`, etc.); avoid god-clients.

## Resolution status (post `feat/sdk`)

Implementation and tests were aligned to this spec in the same branch. Re-run:

- `cargo test -p edgequake-sdk`
- `cd sdks/python && pytest`
- `cd sdks/typescript && bun test`

to verify parity after API changes.
