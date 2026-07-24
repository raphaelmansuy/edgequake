# `PKG-aws-lc-sys` — AWS-LC cryptographic validation flaws

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 5  
> **Laws**: LAW-15, LAW-19, LAW-20  
> **Dependabot**: #80, #82, #84, #117, #119  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: S-sdk / R-prod via TLS. Five high GHSAs: PKCS7 verify bypasses, X.509 name constraints bypass, AES-CCM timing, CRL DP scope logic.

Alerted on **`sdks/rust/Cargo.lock`**. Workspace auth uses `jsonwebtoken` + `aws_lc_rs` which may pull `aws-lc-sys` as well — verify both trees after update.

---

## 2. Advisories

| GHSA | Theme | Floor |
|------|-------|-------|
| GHSA-394x-vwmw-crm3 | PKCS7 chain | ≥0.38 / **0.39** family |
| GHSA-65p9-r9h6-22vj | PKCS7 signature | ≥0.38 |
| GHSA-9f94-5g5w-gf6r | AES-CCM timing | ≥0.38 |
| GHSA-hfpc-8r3f-gw53 | Name constraints | **≥0.39.0** |
| GHSA-vw5v-4f2q-w9xf | CRL DP scope | **≥0.39.0** |

**Security floor**: **`≥0.39.0`**.

**Latest crates.io (audit day)**: `0.43.0` — prefer latest compatible via `cargo update` (LAW-15 satisfied by ≥0.39).

---

## 3. Upgrade steps

```bash
cd sdks/rust
cargo update -p aws-lc-sys
cargo tree -p aws-lc-sys -i
cargo test

# Also check workspace if linked:
cd ../../edgequake
cargo tree -i aws-lc-sys
cargo update -p aws-lc-sys   # if present and <0.39
```

Do not pin an ancient `aws-lc-sys` in `[patch]` or Cargo.toml unless required for MSRV — prefer resolver.

---

## 4. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Native build failures after bump | CI matrix macOS/Linux; document openssl vs aws-lc |
| EC-2 | Parent crate caps version | Bump rustls / aws-lc-rs parents |

---

## 5. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_aws_lc_039` | aws-lc-sys ≥0.39.0; rust SDK tests pass |

Expected close: **#80, #82, #84, #117, #119**.

---

## 6. Cross-refs

Wave 5 · [PKG-jsonwebtoken](PKG-jsonwebtoken.md) · Register `aws-lc-sys`
