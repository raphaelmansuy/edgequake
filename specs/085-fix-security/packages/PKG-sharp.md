# `PKG-sharp` — libvips inherited CVEs

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 6 (may improve after Next/Astro bumps)  
> **Laws**: LAW-15, LAW-19, LAW-20  
> **Dependabot**: #374, #377  
> **Verified against**: v0.21.1 / 2026-07-24  
> **Advisory**: [GHSA-f88m-g3jw-g9cj](https://github.com/advisories/GHSA-f88m-g3jw-g9cj)

---

## 1. WHY

**Class**: R-ssr when Next image optimization / Astro assets use sharp. Inherited libvips CVEs (CVE-2026-33327/33328/35590/35591).

Resolved today: **0.34.5** (transitive). Floor: **`≥0.35.0`**. Latest npm: `0.35.3`.

---

## 2. Upgrade steps

```json
// pnpm.overrides in webui + website
"sharp": ">=0.35.0"
```

```bash
pnpm install
pnpm why sharp
# native rebuild may run — ensure CI has build tools
pnpm run build
```

WebUI already lists `sharp` in `trustedDependencies` / `ignoreScripts` — verify install still produces a working binary after override.

---

## 3. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_sharp_035` | sharp ≥0.35.0; image/build smoke |

Expected close: **#374, #377**.

---

## 4. Cross-refs

Wave 6 · [PKG-next](PKG-next.md) · [PKG-astro](PKG-astro.md)
