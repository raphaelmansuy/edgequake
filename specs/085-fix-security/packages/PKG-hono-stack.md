# `PKG-hono-stack` — hono + `@hono/node-server`

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 3  
> **Laws**: LAW-15, LAW-16, LAW-17, LAW-19, LAW-20  
> **Dependabot**: hono #319–#323, #379–#381 · `@hono/node-server` #378  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: R-prod (MCP HTTP adapter). Transitive via `@modelcontextprotocol/sdk`. Issues include CORS wildcard+credentials, path traversal on Windows serve-static, Lambda body-limit bypass, JSX XSS/`cx()`, cross-request JSX context leak, header de-dup bugs.

Treat **hono + `@hono/node-server`** as one upgrade unit (DRY).

---

## 2. Advisories

### hono

| Floor | Covers |
|-------|--------|
| ≥4.12.25 | CORS, serve-static, Lambda body/cookies/headers family |
| **≥4.12.27** | JSX context isolation, `cx()` XSS, API Gateway header de-dup |

**hono floor**: **`≥4.12.27`**. Latest npm: `4.12.31`.

### `@hono/node-server`

| GHSA | Sev | Patched |
|------|-----|---------|
| [GHSA-frvp-7c67-39w9](https://github.com/advisories/GHSA-frvp-7c67-39w9) | medium | **2.0.5** |

Current resolved: **1.19.13** → needs major bump to **≥2.0.5** (coordinate with MCP SDK compatibility).

---

## 3. Current pins

| Package | Resolved (mcp lock) | Direct? |
|---------|---------------------|---------|
| `hono` | 4.12.23 | transitive |
| `@hono/node-server` | 1.19.13 | transitive |

---

## 4. Target

| Field | Value |
|-------|-------|
| hono | **`≥4.12.27`** (prefer latest 4.12.x) |
| `@hono/node-server` | **`≥2.0.5`** |
| Strategy | Prefer bumping `@modelcontextprotocol/sdk` if it pulls patched pair; else npm `overrides` |

---

## 5. Upgrade steps

```bash
cd mcp
# Try parent first
npm update @modelcontextprotocol/sdk
npm ls hono @hono/node-server

# If still below floor, package.json:
# "overrides": {
#   "hono": ">=4.12.27",
#   "@hono/node-server": ">=2.0.5"
# }
npm install
npm test
```

Also apply companion floors: `fast-uri≥3.1.4`, `body-parser≥2.3.0`, `ip-address≥10.1.1` ([PKG-transitive-npm](PKG-transitive-npm.md)).

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | `@hono/node-server` 2.x breaks MCP SDK | Pin compatible SDK version; if blocked, document PARTIAL + isolate serve-static |
| EC-2 | CORS default wildcard | Explicit allowlist in MCP server config |
| EC-3 | Non-Windows deploy | Path traversal still patched; upgrade anyway |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_hono_41227` | hono ≥4.12.27; node-server ≥2.0.5; tests green |

Expected close: **#319–#323, #378–#381**.

---

## 8. Cross-refs

Wave 3 · [PKG-transitive-npm](PKG-transitive-npm.md) · Register `hono` / `@hono/node-server`
