# `PKG-dompurify` — XSS sanitizer bypass family

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 1  
> **Laws**: LAW-15, LAW-16, LAW-20  
> **Dependabot**: #310–#316, #327, #376  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: R-prod. WebUI sanitizes untrusted HTML/markdown via DOMPurify (`^3.4.0`). Nine GHSAs cover IN_PLACE bypasses, hook pollution, Trusted Types, custom elements, SAFE_FOR_TEMPLATES.

One advisory ([GHSA-x4vx-rjvf-j5p4](https://github.com/advisories/GHSA-x4vx-rjvf-j5p4)) listed `first_patched: null` at audit time with range `≤3.4.6` — pin **latest 3.4.x** and re-check advisory when closing.

---

## 2. Advisories

| Floor progression | Versions |
|-------------------|----------|
| Intermediate | 3.4.6 → 3.4.7 → 3.4.8 → 3.4.9 → 3.4.11 |
| **LAW-15 floor** | **`≥3.4.12`** (covers GHSA-c2j3 + prior) |

**Latest npm (audit day)**: `3.4.12`.

---

## 3. Current pins

| Field | Value |
|-------|-------|
| Direct | `edgequake_webui/package.json` → `"dompurify": "^3.4.0"` |
| Resolved | 3.4.0 in pnpm lock (bun.lock may already show 3.4.12 — **ignore bun**) |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target | **`dompurify@^3.4.12`** |
| Usage policy | Prefer string sanitize; avoid IN_PLACE on attacker-controlled DOM roots |

---

## 5. Upgrade steps

```bash
cd edgequake_webui
pnpm add dompurify@^3.4.12
pnpm why dompurify
pnpm test
```

Audit call sites for `IN_PLACE` / custom hooks mutating `allowedTags`.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | GHSA without patched version | Stay on latest 3.4.x; re-query advisory |
| EC-2 | Hook pollution patterns | Do not mutate config objects in hooks |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_dompurify_3412` | ≥3.4.12; sanitize unit paths pass |

Expected close: **#310–#316, #327, #376**.

---

## 8. Cross-refs

Wave 1 · Register `dompurify`
