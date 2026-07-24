# `PKG-astro` — Website XSS/SSRF → Astro 7.1+

> **Priority**: P0  
> **Audit status**: OPEN  
> **Wave**: 2  
> **Laws**: LAW-15, LAW-16, LAW-17, LAW-18, LAW-20  
> **Dependabot**: #324–#326, #356–#359, #361–#362  
> **Verified against**: v0.21.1 / 2026-07-24  
> **Migration**: [Upgrade to Astro v7](https://docs.astro.build/en/guides/upgrade-to/v7/)

---

## 1. WHY

**Class**: R-ssr (marketing/docs site). Six GHSAs: XSS (slots, spreads, transitions), Host-header SSRF on prerendered errors.

Partial 6.x patches exist (`6.3.3`, `6.4.6`) but **GHSA-4g3v-8h47-v7g6** (View Transition animation XSS) requires **≥7.1.0**. Staying on 6.x cannot satisfy LAW-15.

---

## 2. Advisories

| GHSA | Sev | Patched (relevant) |
|------|-----|---------------------|
| [GHSA-8hv8-536x-4wqp](https://github.com/advisories/GHSA-8hv8-536x-4wqp) | high | 6.3.3+ |
| [GHSA-2pvr-wf23-7pc7](https://github.com/advisories/GHSA-2pvr-wf23-7pc7) | high | 6.4.6+ |
| [GHSA-jrpj-wcv7-9fh9](https://github.com/advisories/GHSA-jrpj-wcv7-9fh9) | medium | 6.4.6+ |
| [GHSA-f48w-9m4c-m7f5](https://github.com/advisories/GHSA-f48w-9m4c-m7f5) | medium | 7.0.6+ |
| [GHSA-7pw4-f3q4-r2p2](https://github.com/advisories/GHSA-7pw4-f3q4-r2p2) | low | 7.0.4+ |
| [GHSA-4g3v-8h47-v7g6](https://github.com/advisories/GHSA-4g3v-8h47-v7g6) | medium | **7.1.0** |

**Security floor**: **`≥7.1.0`**.

**Latest npm (audit day)**: `7.1.3`.

---

## 3. Current pins

| Field | Value |
|-------|-------|
| Direct | `edgequake-website/package.json` → `astro@^6.1.10` |
| Lock | `edgequake-website/pnpm-lock.yaml` |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target | **`astro@^7.1.3`** (or latest 7.1.x) |
| Why major required | Floor for GHSA-4g3v is 7.1.0 |
| Breaking | Vite 8, Rust compiler (stricter HTML), `compressHTML: 'jsx'`, reserved `src/fetch.ts`, Sätteri markdown default |

---

## 5. Upgrade steps

1. `cd edgequake-website && pnpm dlx @astrojs/upgrade`  
2. Walk v7 guide checklist; fix unclosed tags / invalid nesting.  
3. Decide `compressHTML: true` if spacing regressions appear.  
4. If `src/fetch.ts` exists for non-routing use → rename or set `fetchFile`.  
5. `pnpm run build` + visual spot-check.  
6. Re-check pulled `svgo` / `sharp` / `js-yaml` / `vite` floors.

Isolated PR (LAW-17) — do not mix with Next bump.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | remark/rehype plugins | Install `@astrojs/markdown-remark` + `unified()` if needed |
| EC-2 | Integration `getContainerRenderer` | Use `/container-renderer` entry |
| EC-3 | Static site low exploitability | Still upgrade — LAW-15 |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_astro_71` | astro ≥7.1.0; build green; checklist done |

Expected close: Astro alerts **#324–#362** set.

---

## 8. Cross-refs

Wave 2 · [PKG-vite](PKG-vite.md) · [PKG-sharp](PKG-sharp.md) · [PKG-js-yaml](PKG-js-yaml.md) · Register `astro`
