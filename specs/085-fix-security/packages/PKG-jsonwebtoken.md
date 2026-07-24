# `PKG-jsonwebtoken` — Type confusion (exp/nbf) + dual-pin SSOT

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 5  
> **Laws**: LAW-15, LAW-16, LAW-21, LAW-20  
> **Dependabot**: #15, #348  
> **Verified against**: v0.21.1 / 2026-07-24  
> **Advisory**: [GHSA-h395-gr6q-cpjc](https://github.com/advisories/GHSA-h395-gr6q-cpjc) / CVE-2026-25537

---

## 1. WHY

**Class**: R-prod (auth). Malformed string `exp`/`nbf` can be treated as absent when validation flags are on but claims are not in `required_spec_claims` → time-claim bypass.

**Dual pin (LAW-21 violation today)**:

| Location | Declared | Resolved |
|----------|----------|----------|
| `edgequake-auth` | `10.3` + `aws_lc_rs` | **10.4.0** (patched) |
| `edgequake-api` `[dev-dependencies]` | `9.3` | **9.3.1** (vulnerable) |
| workspace `Cargo.toml` | `9.3` | pulls 9.x |

Dependabot still flags API + lock because **9.x remains in the graph**.

---

## 2. Advisories

| GHSA | Sev | Patched |
|------|-----|---------|
| GHSA-h395-gr6q-cpjc | medium | **≥10.3.0** |

**Latest crates.io (audit day)**: `10.4.0`.

---

## 3. Target

| Field | Value |
|-------|-------|
| Target | **Single pin ≥10.3** (prefer `10.3`/`10.4` with `aws_lc_rs`) |
| Delete | workspace + api `jsonwebtoken = "9.3"` |
| Auth SSOT | `edgequake-auth` owns runtime JWT |

---

## 4. Upgrade steps

1. Remove `jsonwebtoken` from `[workspace.dependencies]` **or** set to `{ version = "10.3", features = ["aws_lc_rs"] }`.  
2. Change `edgequake-api` dev-dep to `10.3` + features (or depend on auth test helpers).  
3. `cargo update -p jsonwebtoken`  
4. `cargo tree -i jsonwebtoken` → **no 9.x**.  
5. `cargo test -p edgequake-auth` (+ API tests that mint/verify JWTs).  
6. Ensure Validation rejects non-numeric exp/nbf (covered by 10.3+).

---

## 5. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Feature `aws_lc_rs` required | Keep explicit feature (10.x crypto backend) |
| EC-2 | aws-lc-sys advisories | See [PKG-aws-lc-sys](PKG-aws-lc-sys.md) |

---

## 6. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_jwt_103` | no 9.x in tree; auth tests pass |

Expected close: **#15, #348**.

---

## 7. Cross-refs

Wave 5 · [PKG-aws-lc-sys](PKG-aws-lc-sys.md) · Register `jsonwebtoken`
