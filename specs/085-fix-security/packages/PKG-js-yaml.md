# `PKG-js-yaml` — Merge-key quadratic DoS

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 6  
> **Laws**: LAW-15, LAW-18, LAW-19, LAW-20  
> **Dependabot**: #347, #349, #360, #366  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: T-build / D-dev (eslint, astro, openapi tooling). Crafted YAML with merge-key chains / repeated aliases can force quadratic CPU.

GHSAs:

- [GHSA-h67p-54hq-rp68](https://github.com/advisories/GHSA-h67p-54hq-rp68) → patched **4.2.0**  
- [GHSA-52cp-r559-cp3m](https://github.com/advisories/GHSA-52cp-r559-cp3m) → patched **4.3.0**

**Floor**: **`≥4.3.0`**.

**Latest npm (audit day)**: `5.2.2` — **do not** force js-yaml 5 via overrides unless parents support it (LAW-18). Prefer `>=4.3.0 <5`.

---

## 2. Upgrade steps

```json
"js-yaml": ">=4.3.0"
```

Apply in webui + website; regenerate locks; `pnpm why js-yaml`.

---

## 3. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_js_yaml_430` | all js-yaml ≥4.3.0 |

Expected close: **#347, #349, #360, #366**.

---

## 4. Cross-refs

Wave 6 · [PKG-astro](PKG-astro.md) · [PKG-transitive-npm](PKG-transitive-npm.md)
