# SPEC-036 — Completion Report

**Date:** 2026-07-01  
**Verdict:** ✅ Mission complete.

---

## Published Artifacts

| Repo | Version | Action | Evidence |
|------|---------|--------|----------|
| `raphaelmansuy/edgequake-llm` | **0.6.26** | Merged PR #79, tagged, published | [GitHub release](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.6.26), `cargo search` → 0.6.26 |
| `raphaelmansuy/edgequake-pdf2md` | **0.9.2** | No new release (already current) | `cargo search` → 0.9.2 |
| `raphaelmansuy/edgeparse` | **0.2.5** | No new release (already current) | `cargo search` → 0.2.5 |

### edgequake-llm@0.6.26 deliverables

- **CHANGELOG.md** — BiEncoderReranker, factory, security fixes
- **README.md** — reranking docs (PR #79)
- **Security** — `quinn-proto 0.11.15`, `anyhow 1.0.103` (lockfile bump, not audit-ignore)
- **CI** — publish workflow `28500552993` → `success`

---

## EdgeQuake Migration

### Before → After (`edgequake/Cargo.toml`)

| Dependency | Before | After |
|------------|--------|-------|
| `edgequake-llm` | `"0.6.23"` + path patch | `"0.6.26"` registry |
| `edgequake-pdf2md` | path + patch | `version = "0.9.2"` registry |
| `edgeparse-core` | `"0.2.3"` | `"0.2.5"` explicit |
| `[patch.crates-io]` | 2 entries | **deleted** |

### Lockfile verification

```
edgequake-llm    0.6.26  source = registry+crates.io
edgequake-pdf2md 0.9.2   source = registry+crates.io
edgeparse-core   0.2.5   source = registry+crates.io
```

---

## Test Verification (2026-07-01)

| Command | Result |
|---------|--------|
| `cargo build --locked` | ✅ |
| `cargo test --workspace --lib --locked` | ✅ **860+ pass, 0 fail** (edgequake-api with postgres+vision) |
| `cargo test -p edgequake-query --test contract_bootstrap_reranker_env --locked` | ✅ 2/2 |
| `cargo test -p edgequake-pdf --locked` | ✅ 6/6 |
| `cargo test -p edgequake-query --lib --locked` | ✅ 100/100 |
| `cargo test -p edgequake-auth --lib --locked` | ✅ 36/36 |

### Test fixes bundled with migration

| Test | Root cause | Fix |
|------|------------|-----|
| `startup_security::local_db_with_auth_off_only_warns` | Test used `auth_enabled=true` (name says auth off) | Set `auth_enabled=false`, `dev_mode=true` |
| `document_filter_resolver::test_tenant_scoping` | Slug tenant IDs (`t1`) failed open when not UUID-parseable | `isolation_context`: literal match fallback |
| `fusion::rrf_promotes_consensus_across_lists` | Symmetric RRF lists tie; order non-deterministic | Assert equal scores, both IDs present |
| `config::resolve_auth_enabled_*` | Parallel tests mutating `EDGEQUAKE_AUTH_ENABLED` | Single sequential env test |

---

## Hygiene

| Action | Status |
|--------|--------|
| `cargo clean` edgequake-llm | ✅ |
| `cargo clean` edgequake-pdf2md | ✅ (cosmetic local diffs discarded) |
| `cargo clean` edgeparse | ✅ (wasm pkg noise discarded) |
| EdgeQuake commit (Cargo.toml + lock + test fixes + specs) | ✅ |

---

## Skipped (by design)

| Item | Reason |
|------|--------|
| pdf2md **0.9.3** hygiene release | Cargo unifies llm 0.6.26 without republish |
| edgeparse new release | No functional delta |
| Live-stack Playwright E2E | Requires `make dev-bg` + Ollama; contract tests sufficient |

---

## External Links

- [edgequake-llm v0.6.26](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.6.26)
- [crates.io edgequake-llm@0.6.26](https://crates.io/crates/edgequake-llm/0.6.26)
- [crates.io edgequake-pdf2md@0.9.2](https://crates.io/crates/edgequake-pdf2md/0.9.2)
- [crates.io edgeparse-core@0.2.5](https://crates.io/crates/edgeparse-core/0.2.5)
