# SPEC-036 — Cross-Reference Matrix

**Updated:** 2026-07-01 (post-migration)

---

## Post-migration claims

| Claim | Evidence | Verified |
|-------|----------|----------|
| No `[patch.crates-io]` in EdgeQuake | `edgequake/Cargo.toml` — section absent | ✅ |
| llm pin = 0.6.26 registry | `Cargo.toml` L152; lock L1932–1934 `source = registry` | ✅ |
| pdf2md pin = 0.9.2 registry | `Cargo.toml` L159; lock L2005–2007 | ✅ |
| edgeparse pin = 0.2.5 registry | `Cargo.toml` L158; lock L1729–1731 | ✅ |
| llm 0.6.26 on crates.io | `cargo search edgequake-llm` | ✅ |
| GitHub release v0.6.26 | `gh release view v0.6.26` | ✅ |
| Publish CI success | run `28500552993` conclusion=success | ✅ |
| Reranker factory from registry | `contract_bootstrap_reranker_env` 2/2 `--locked` | ✅ |
| pdf backends from registry | `cargo test -p edgequake-pdf` 6/6 `--locked` | ✅ |
| Security: quinn-proto 0.11.15 | `edgequake-llm/Cargo.lock` + CHANGELOG Security | ✅ |
| Security: anyhow 1.0.103 | same | ✅ |

---

## Pre-migration claims (historical)

| Claim | Evidence | Was |
|-------|----------|-----|
| EdgeQuake patched llm locally | old `Cargo.toml` L183–185 | ✅ fixed |
| EdgeQuake path-dep pdf2md | old `Cargo.toml` L160 | ✅ fixed |
| llm 0.6.26 unpublished | PR #79 open | ✅ fixed |
| Factory absent on 0.6.25 | bootstrap.rs vs crates.io 0.6.25 | ✅ fixed |

---

## Requirement traceability (final)

| REQ | Description | Status |
|-----|-------------|--------|
| REQ-036-01 | llm 0.6.26 on crates.io | ✅ |
| REQ-036-02 | pdf2md from registry | ✅ |
| REQ-036-03 | edgeparse 0.2.5 explicit pin | ✅ |
| REQ-036-04 | No `[patch.crates-io]` | ✅ |
| REQ-036-05 | CHANGELOG per release | ✅ llm only |
| REQ-036-06 | README current | ✅ |
| REQ-036-07 | E2E after migration | ⚠️ contract tests only |
| REQ-036-08 | Clean-room build | ⚠️ lockfile proof (clone aborted) |

---

## PR traceability (final)

| Repo | PR / Release | Version | Status |
|------|-------------|---------|--------|
| edgequake-llm | PR #79 → v0.6.26 | 0.6.26 | ✅ merged + published |
| edgequake-pdf2md | — | 0.9.2 | ✅ reused |
| edgeparse | — | 0.2.5 | ✅ reused |
| edgequake | pending commit | 0.12.11 | ⏳ local changes only |

---

## Security fix traceability

| Advisory | Severity | Fix applied | Method |
|----------|----------|-------------|--------|
| RUSTSEC-2026-0185 | HIGH 7.5 | quinn-proto 0.11.15 | `cargo update -p quinn-proto --precise 0.11.15` |
| RUSTSEC-2026-0190 | unsound | anyhow 1.0.103 | `cargo update -p anyhow` |
| RUSTSEC-2026-0097 | unsound (rand) | — | warning only; not applicable (no custom logger) |
| RUSTSEC-2025-0012 | unmaintained (backoff) | — | `.cargo/audit.toml` ignore (no fix upstream) |

**First-principles rule:** HIGH/critical vulnerabilities → upgrade deps. Unmaintained with no alternative → documented ignore.

---

## DRY / coupling audit (final)

| Before | After |
|--------|-------|
| 2 path patches + 1 path dep | 0 patches, 3 version pins |
| External contributors cannot build | `cargo build --locked` from registry |
| Comment drift ("until 0.6.26") | Pin matches published version |
