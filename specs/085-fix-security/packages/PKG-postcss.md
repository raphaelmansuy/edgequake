# `PKG-postcss` — sourceMappingURL file read + style XSS

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 6  
> **Laws**: LAW-15, LAW-16, LAW-19, LAW-20  
> **Dependabot**: #215, #217, #219, #408–#410  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: T-build / D-dev (and CSS pipeline). Two GHSAs:

- XSS via unescaped `</style>` in stringify output (patched **8.5.10**)  
- Arbitrary file read via attacker-controlled `sourceMappingURL` in CSS comments (patched **8.5.12**)

Appears in webui, website, ts-sdk locks (often nested under next/vite/tailwind).

---

## 2. Floor

**`≥8.5.12`** (LAW-15 max of patched versions).

**Latest npm (audit day)**: `8.5.22` — overrides may use `>=8.5.12` or pin latest 8.5.x.

---

## 3. Upgrade steps

```json
// each affected package.json pnpm/npm overrides
"postcss": ">=8.5.12"
```

```bash
pnpm install   # or npm install
pnpm why postcss
```

Never leave a nested `postcss@8.4.x` / `8.5.6` / `8.5.11` in the tree.

---

## 4. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_postcss_8512` | all resolved postcss ≥8.5.12 |

Expected close: **#215–#219, #408–#410**.

---

## 5. Cross-refs

Wave 6 · [PKG-vite](PKG-vite.md) · [PKG-next](PKG-next.md) · [PKG-transitive-npm](PKG-transitive-npm.md)
