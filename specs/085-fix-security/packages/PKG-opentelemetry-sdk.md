# `PKG-opentelemetry-sdk` — Unbounded W3C Baggage allocation

> **Priority**: P2  
> **Audit status**: FIXED  
> **Wave**: 5  
> **Laws**: LAW-15, LAW-16, LAW-20  
> **Dependabot**: #345, #346  
> **Verified against**: v0.21.1 / 2026-07-24  
> **Advisory**: [GHSA-w9wp-h8wv-79jx](https://github.com/advisories/GHSA-w9wp-h8wv-79jx) / CVE-2026-48504

---

## 1. WHY

**Class**: R-prod when OTEL feature enabled. Unbounded memory allocation while propagating W3C Baggage → DoS via crafted baggage headers.

Declared in `edgequake-observability` as **`0.32`**; floor is **`0.32.1`**. Lock resolves a single **`opentelemetry_sdk@0.32.1`**.

---

## 2. Advisories

| GHSA                | Sev    | Patched     |
| ---------------------| --------| -------------|
| GHSA-w9wp-h8wv-79jx | medium | **≥0.32.1** |

**Latest crates.io (audit day)**: `0.32.1` (matches floor).

**Residual cleared (2026-07-24)**: Published `edgequake-pdf2md@0.9.8` depends on `edgequake-llm@0.10.2` (OTEL 0.32 / `tracing-opentelemetry` 0.33). Workspace pin `edgequake-pdf2md = "0.9.8"`; `Cargo.lock` no longer contains `opentelemetry_sdk@0.27.1` or `edgequake-llm@0.6.x`.

---

## 3. Current pins

```toml
# edgequake-observability/Cargo.toml
opentelemetry = { version = "0.32", optional = true }
opentelemetry_sdk = { version = "0.32", features = ["rt-tokio"], optional = true }
opentelemetry-otlp = { version = "0.32", features = ["grpc-tonic"], optional = true }
tracing-opentelemetry = { version = "0.33", optional = true }
```

Workspace: `edgequake-pdf2md = "0.9.8"`, `edgequake-llm = "0.10.2"`.

---

## 4. Target

Coordinated bump (LAW-16) — **landed**:

| Crate | Target | Lock |
|-------|--------|------|
| `opentelemetry` | `0.32` | 0.32.0 |
| `opentelemetry_sdk` | **`0.32`** (≥0.32.1) | **0.32.1** |
| `opentelemetry-otlp` | `0.32` | 0.32.0 |
| `tracing-opentelemetry` | compatible with otel 0.32 | 0.33.0 |

No leftover 0.27 in `cargo tree` / `Cargo.lock`.

---

## 5. Upgrade steps

1. Edit observability crate versions together.  
2. `cargo update -p opentelemetry_sdk -p opentelemetry -p opentelemetry-otlp`  
3. Fix compile breaks in exporters/resource builders.  
4. Publish `edgequake-pdf2md` ≥0.9.8 (llm ≥0.10.2) so the diamond cannot reintroduce 0.27.  
5. `cargo test -p edgequake-observability --features otel`  
6. `cargo tree -p edgequake-observability --features otel -i opentelemetry_sdk`

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Feature off in default builds | Still bump — Dependabot + optional feature users |
| EC-2 | Companion crate lag | Block merge until tree has no ≤0.32.0 sdk; pdf2md must stay on llm ≥0.10.2 |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_otel_0321` | sdk ≥0.32.1; feature tests pass |

Expected close: **#345, #346** (auto-dismiss after lock with only `0.32.1` lands on default branch).

---

## 8. Cross-refs

Wave 5 · Register `opentelemetry_sdk`
