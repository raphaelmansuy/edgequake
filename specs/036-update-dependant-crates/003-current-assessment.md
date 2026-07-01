# SPEC-036 — Current Assessment (Code Is Law)

**Initial audit:** 2026-07-01  
**Post-migration audit:** 2026-07-01  
**Repos:** sibling dirs under `/Users/raphaelmansuy/Github/03-working/`

---

## POST-MIGRATION (final)

### EdgeQuake workspace (`edgequake/Cargo.toml`)

```152:159:edgequake/Cargo.toml
edgequake-llm = "0.6.26"
…
edgeparse-core = "0.2.5"
edgequake-pdf2md = { version = "0.9.2", default-features = false, features = ["bundled"] }
```

**No `[patch.crates-io]` section.** Verified: `grep patch edgequake/Cargo.toml` → empty.

### Cargo.lock resolution (registry-only)

| Package | Lock version | Source |
|---------|-------------|--------|
| `edgequake-llm` | 0.6.26 | `registry+crates.io` ✅ |
| `edgequake-pdf2md` | 0.9.2 | `registry+crates.io` ✅ |
| `edgeparse-core` | 0.2.5 | `registry+crates.io` ✅ |

### Functional API from published crates

`create_production_reranker` resolves from **crates.io `edgequake-llm@0.6.26`**:

```
cargo test -p edgequake-query --test contract_bootstrap_reranker_env --locked
→ 2/2 pass (2026-07-01)
```

---

## PRE-MIGRATION (historical)

<details>
<summary>Original audit snapshot (2026-07-01 morning)</summary>

### Dependency declarations (before)

```toml
edgequake-llm = "0.6.23"          # + [patch.crates-io] path
edgeparse-core = "0.2.3"
edgequake-pdf2md = { path = "../../edgequake-pdf2md", … }
[patch.crates-io]
edgequake-llm = { path = "../../edgequake-llm" }
edgequake-pdf2md = { path = "../../edgequake-pdf2md" }
```

### Gap summary (resolved)

| Gap | Was | Now |
|-----|-----|-----|
| G1 llm 0.6.26 unpublished | P0 blocker | ✅ crates.io |
| G2 `[patch.crates-io]` | masked G1 | ✅ removed |
| G3 pdf2md path dep | dev leftover | ✅ registry 0.9.2 |
| G4 stale llm pin 0.6.23 | comment drift | ✅ 0.6.26 |
| G5 edgeparse pin 0.2.3 | implicit | ✅ explicit 0.2.5 |

</details>

---

## 2. edgequake-llm — PUBLISHED

| Field | Value |
|-------|-------|
| Version | **0.6.26** |
| crates.io | ✅ indexed |
| GitHub release | ✅ [v0.6.26](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.6.26) |
| PR #79 | ✅ merged (squash) |
| Publish CI | ✅ run `28500552993` success |
| Security | `quinn-proto 0.11.15`, `anyhow 1.0.103` in lockfile |
| CHANGELOG | `[0.6.26]` Added + Security sections |
| docs.rs | ⏳ building (404 at completion; crate on crates.io) |

---

## 3. edgequake-pdf2md — NO NEW RELEASE

| Field | Value |
|-------|-------|
| crates.io latest | **0.9.2** (unchanged) |
| EdgeQuake consumption | registry `0.9.2` with `bundled` feature |
| Action taken | None — Option A from implementation plan |

---

## 4. edgeparse — NO NEW RELEASE

| Field | Value |
|-------|-------|
| crates.io latest | **0.2.5** (unchanged) |
| EdgeQuake consumption | registry `0.2.5` explicit pin |
| Action taken | Verify only |

---

## 5. What is NOT a gap (confirmed)

- No unpublished edgeparse features required.
- No unpublished pdf2md code since v0.9.2.
- pdf2md 0.9.3 hygiene release not required (Cargo unifies llm 0.6.26).
- `edgequake-litellm` Python package — out of scope.
