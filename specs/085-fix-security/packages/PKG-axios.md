# `PKG-axios` — Prototype pollution / DoS / proxy bypass

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 1  
> **Laws**: LAW-15, LAW-16, LAW-19, LAW-20  
> **Dependabot**: #350, #352–#355, #363–#364, #367–#369  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: R-prod (browser + any Node adapter paths). Ten GHSAs: prototype pollution gadgets, DoS via deep form serialization, `maxBodyLength` bypasses, NO_PROXY/`0.0.0.0` issues, inherited proxy after interceptor clone.

WebUI uses axios as a **direct** HTTP client (`^1.16.0`).

---

## 2. Advisories

All list first patched **`1.18.0`**. Representative:

| GHSA | Sev | Theme |
|------|-----|-------|
| [GHSA-gcfj-64vw-6mp9](https://github.com/advisories/GHSA-gcfj-64vw-6mp9) | high | Inherited proxy after interceptor cloning |
| [GHSA-mmx7-hfxf-jppx](https://github.com/advisories/GHSA-mmx7-hfxf-jppx) | medium | Prototype pollution gadgets |
| [GHSA-xj6q-8x83-jv6g](https://github.com/advisories/GHSA-xj6q-8x83-jv6g) | medium | Auth subfield pollution → Basic auth |
| … | medium | formData/DoS/maxBodyLength/NO_PROXY family |

**Security floor**: **`≥1.18.0`**.

**Latest npm (audit day)**: `1.18.1` (prefer this).

---

## 3. Current pins

| Field | Value |
|-------|-------|
| Direct | `edgequake_webui/package.json` → `"axios": "^1.16.0"` |
| Resolved | ~1.16.0 in `pnpm-lock.yaml` |
| Child | `form-data@4.0.5` (needs ≥4.0.6 — see Wave 1) |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target | **`axios@^1.18.1`** |
| Why not 2.x | Not required by LAW-15; avoid unforced major |

---

## 5. Upgrade steps

```bash
cd edgequake_webui
pnpm add axios@^1.18.1
pnpm why axios form-data
# if form-data still <4.0.6:
#   add pnpm.overrides "form-data": ">=4.0.6"
pnpm test && pnpm run typecheck
```

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Custom interceptors mutate config | Re-test auth header paths |
| EC-2 | Large uploads | Confirm maxBodyLength still enforced |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_axios_118` | axios ≥1.18.0 |
| `sec085_form_data_406` | form-data ≥4.0.6 |

Expected close: axios alerts **#350–#369** set + form-data **#328**.

---

## 8. Cross-refs

Wave 1 · [PKG-transitive-npm](PKG-transitive-npm.md) (`form-data`) · Register `axios`
