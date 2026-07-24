# `PKG-vite` — Dev-server FS bypass / path traversal (line-aware)

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 6 (after Wave 0/2 settle lines)  
> **Laws**: LAW-15, LAW-17, LAW-18, LAW-19, LAW-20  
> **Dependabot**: #162+ family, #317–#318, #331–#332, #335–#336  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: D-dev (high). Vite dev server FS deny bypasses, websocket arbitrary file read, optimized-deps path traversal, launch-editor NTLM hash disclosure (Windows).

Not production runtime for EdgeQuake API, but CI/dev machines and Vitest pull Vite.

---

## 2. Advisories → line floors

| Line | Floor | Surfaces today |
|------|-------|----------------|
| Vite 6.x | **≥6.4.3** | website (astro6), ts-sdk |
| Vite 7.x | **≥7.3.5** | webui (via vitest), website mixed |
| Vite 8.x | (post Astro 7) | prove no open GHSA after Wave 2 |

Do **not** force all surfaces to Vite 8 unless Astro 7 requires it (LAW-18).

**Latest npm (audit day)**: `8.1.5` — not the universal target.

---

## 3. Current pins (resolved)

| Surface | Vite |
|---------|------|
| webui | ~7.3.1 (needs ≥7.3.5) |
| website | 6.4.1 + 7.3.5 mixed |
| ts-sdk | 6.4.1 (needs ≥6.4.3) |
| mcp | 8.0.16 (verify advisories after install) |

---

## 4. Target

Per-surface overrides:

```json
// webui pnpm.overrides
"vite": ">=7.3.5"

// website (pre-Astro7) / ts-sdk
"vite": ">=6.4.3"
// after Astro7: accept Vite 8 from astro; re-audit
```

---

## 5. Upgrade steps

1. Wave 0: ts-sdk vitest bump may pull vite — enforce 6.4.3+.  
2. Wave 2: Astro 7 may move website to Vite 8 — re-run Dependabot.  
3. Wave 6: apply overrides; `pnpm/npm install`; `pnpm why vite` / `npm ls vite`.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Multiple vite majors in one lock | Accept nested; each instance ≥ its line floor |
| EC-2 | Windows-only GHSAs | Still bump on macOS CI for LAW-15 |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_vite_line` | every resolved vite meets its line floor |

---

## 8. Cross-refs

Wave 6 · [PKG-vitest](PKG-vitest.md) · [PKG-astro](PKG-astro.md) · [PKG-transitive-npm](PKG-transitive-npm.md)
