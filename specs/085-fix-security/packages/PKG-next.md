# `PKG-next` — July 2026 Next.js security release

> **Priority**: P0  
> **Audit status**: OPEN  
> **Wave**: 0  
> **Laws**: LAW-15, LAW-16, LAW-17, LAW-18, LAW-20  
> **Dependabot**: #390–#407 (18 alerts)  
> **Verified against**: v0.21.1 / 2026-07-24  
> **Vendor**: [July 2026 Security Release](https://nextjs.org/blog/july-2026-security-release)

---

## 1. WHY

**Class**: R-prod / R-ssr. Nine GHSAs on App Router / Server Actions / middleware / rewrites / image optimizer — DoS, SSRF, middleware bypass, cache confusion, endpoint disclosure.

EdgeQuake WebUI runs Next **16.2.6** App Router — in the vulnerable range `≥16.0.0 <16.2.11`.

---

## 2. Advisories (floor 16.2.11)

| GHSA | Sev | Summary |
|------|-----|---------|
| [GHSA-m99w-x7hq-7vfj](https://github.com/advisories/GHSA-m99w-x7hq-7vfj) | high | DoS via Server Actions |
| [GHSA-6gpp-xcg3-4w24](https://github.com/advisories/GHSA-6gpp-xcg3-4w24) | high | Middleware/Proxy bypass (Turbopack + single locale) |
| [GHSA-p9j2-gv94-2wf4](https://github.com/advisories/GHSA-p9j2-gv94-2wf4) | high | SSRF in rewrites via attacker-controlled hostname |
| [GHSA-89xv-2m56-2m9x](https://github.com/advisories/GHSA-89xv-2m56-2m9x) | high | SSRF in Server Actions on custom servers |
| [GHSA-q8wf-6r8g-63ch](https://github.com/advisories/GHSA-q8wf-6r8g-63ch) | medium | Image Optimization DoS via SVG |
| [GHSA-4c39-4ccg-62r3](https://github.com/advisories/GHSA-4c39-4ccg-62r3) | medium | Unbounded Server Action payload (Edge) |
| [GHSA-955p-x3mx-jcvp](https://github.com/advisories/GHSA-955p-x3mx-jcvp) | medium | Unauthenticated Server Function endpoint disclosure |
| [GHSA-68g3-v927-f742](https://github.com/advisories/GHSA-68g3-v927-f742) | medium | Cache confusion (request bodies) |
| [GHSA-4633-3j49-mh5q](https://github.com/advisories/GHSA-4633-3j49-mh5q) | medium | Cache confusion (invalid UTF-8) |

**Security floor**: **`16.2.11`** (Active LTS). Also patched on 15.5.21 (N/A — we are on 16.x).

**Latest npm (audit day)**: `16.2.11`.

---

## 3. Current pins

| Field | Value |
|-------|-------|
| Direct | `edgequake_webui/package.json` → `"next": "16.2.6"` |
| Dev align | `eslint-config-next`: `16.2.4` (bump with next) |
| Lock | `pnpm-lock.yaml` |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target | **`next@16.2.11`** (exact or `16.2.11`) |
| Why not 16.3 canary | LAW-18 — canary not required; LTS patch closes all nine GHSAs |
| Related | may pull newer `sharp` — still enforce sharp ≥0.35.0 in Wave 6 if needed |

---

## 5. Upgrade steps

```bash
cd edgequake_webui
pnpm add next@16.2.11
pnpm add -D eslint-config-next@16.2.11
pnpm run typecheck && pnpm test && pnpm run build
```

Playbook: [05-surface-playbooks.md](../05-surface-playbooks.md) §1.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Custom `rewrites()` from request host | Audit next.config; prefer static destinations |
| EC-2 | Server Actions enabled | Upgrade mandatory; no workaround per vendor |
| EC-3 | Image optimizer remote SVG | Keep remotePatterns tight; upgrade |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_next_16_2_11` | resolved next ≥16.2.11; typecheck/test/build green |

Expected close: **#390–#407**.

---

## 8. Cross-refs

Wave 0 · [PKG-sharp](PKG-sharp.md) · Register `next`
