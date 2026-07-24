# `PKG-vitest` — Critical UI-server RCE / file read

> **Priority**: P0  
> **Audit status**: OPEN  
> **Wave**: 0  
> **Laws**: LAW-15, LAW-16, LAW-17, LAW-20  
> **Dependabot**: #301  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: D-dev (Critical). When the Vitest **UI server** is listening, an attacker who can reach it can read/execute arbitrary files ([GHSA-5xrq-8626-4rwp](https://github.com/advisories/GHSA-5xrq-8626-4rwp)).

EdgeQuake: alert is on `sdks/typescript` (`vitest@^3.0.0`). WebUI/MCP already declare `^4.1.0` (patched on the 4.x line).

---

## 2. Advisories

| GHSA | Sev | Vulnerable | Patched |
|------|-----|------------|---------|
| [GHSA-5xrq-8626-4rwp](https://github.com/advisories/GHSA-5xrq-8626-4rwp) | critical | `< 3.2.6` | **3.2.6** |
| same | critical | `≥4.0.0 <4.1.0` | **4.1.0** |

**Security floor (LAW-15)**: `≥3.2.6` on 3.x **or** `≥4.1.0` on 4.x.

**Latest npm (audit day)**: `4.1.10`.

---

## 3. Current pins

| Surface | Declared | Notes |
|---------|----------|-------|
| `sdks/typescript` | `^3.0.0` | **Vulnerable** if resolved <3.2.6 |
| `edgequake_webui` | `^4.1.0` | Already ≥ floor for 4.x |
| `mcp` | `^4.1.0` | Already ≥ floor for 4.x |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target (ts-sdk) | `vitest@^3.2.6` **or** align to `^4.1.10` |
| Preference | Prefer **`^3.2.6`** for minimal churn (LAW-18); optional align to 4.1.x for repo DRY |
| Why not required major | 3.2.6 closes the Critical alert on this lockfile |

---

## 5. Upgrade steps

1. `cd sdks/typescript && npm install -D vitest@^3.2.6` (or `@^4.1.10`).  
2. `npm test`.  
3. Prove: `npm ls vitest`.  
4. Do not expose Vitest UI on untrusted networks (defense in depth).

See [05-surface-playbooks.md](../05-surface-playbooks.md) §4.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | CI uses Vitest UI | Disable UI in CI; bind localhost only |
| EC-2 | Dual 3.x/4.x in monorepo | Acceptable; document; optional later unify |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_vitest_floor` | ts-sdk vitest ≥3.2.6 (or ≥4.1.0); tests pass |

Expected close: **#301**.

---

## 8. Cross-refs

Wave 0 · [PKG-vite](PKG-vite.md) (vitest pulls vite) · Register row `vitest`
