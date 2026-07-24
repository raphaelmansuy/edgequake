# `PKG-transitive-npm` — Remaining npm transitives (DRY override pack)

> **Priority**: P1–P2  
> **Audit status**: OPEN  
> **Wave**: 3 (MCP subset) + **6** (all)  
> **Laws**: LAW-15, LAW-16, LAW-19, LAW-20  
> **Verified against**: v0.21.1 / 2026-07-24

One study for all remaining npm packages that are **lockfile-only**. Apply floors via `pnpm.overrides` / npm `overrides`; never hand-edit locks.

---

## 1. Floor table

| Package | Max sev | Floor | Surfaces | Alerts (sample) | Notes |
|---------|---------|-------|----------|-----------------|-------|
| `form-data` | high | **≥4.0.6** | webui | #328 | Follows axios Wave 1 |
| `fast-uri` | high | **≥3.1.4** | mcp, website | #372–#375, #382–#383 | via ajv |
| `brace-expansion` | high | **≥2.1.2** (2.x) / **≥5.0.7** (3–5.x) | webui, ts-sdk | #351, #365, #371 | line-aware |
| `picomatch` | high | **≥4.0.4** | website, ts-sdk | (register) | ReDoS / method injection |
| `minimatch` | high | **≥9.0.7** | webui, ts-sdk | (register) | webui already overrides `>=3.1.5` — **raise** to close GHSA |
| `rollup` | high | **≥4.59.0** | mcp, ts-sdk | (register) | webui already has override |
| `flatted` | high | **≥3.4.2** | webui | (register) | prototype pollution parse |
| `svgo` | high | **≥4.0.2** | website | #373 | removeScripts incomplete |
| `esbuild` | low | **≥0.28.1** | website, webui, mcp, ts-sdk | (register) | Windows dev-server file read |
| `@babel/core` | low | **≥7.29.6** | webui, website | #333–#334 | sourceMappingURL file read |
| `body-parser` | low | **≥2.3.0** | mcp | #370 | invalid limit disables size |
| `ip-address` | medium | **≥10.1.1** | mcp | (register) | XSS in Address6 HTML helpers |
| `smol-toml` | medium | **≥1.6.1** | website | (register) | DoS via comment lines |
| `yaml` | medium | **≥2.8.3** | website | (register) | stack overflow nested collections |

Also covered by dedicated studies (do not duplicate overrides blindly — compose):  
`postcss`, `sharp`, `js-yaml`, `vite`, `hono`, `@hono/node-server`.

---

## 2. Recommended override blocks

### `edgequake_webui` (`pnpm.overrides`) — extend existing

```json
{
  "lodash-es": "4.18.1",
  "follow-redirects": "^1.16.0",
  "rollup": ">=4.59.0",
  "minimatch": ">=9.0.7",
  "postcss": ">=8.5.12",
  "sharp": ">=0.35.0",
  "js-yaml": ">=4.3.0",
  "brace-expansion": ">=2.1.2",
  "form-data": ">=4.0.6",
  "flatted": ">=3.4.2",
  "vite": ">=7.3.5",
  "@babel/core": ">=7.29.6",
  "esbuild": ">=0.28.1"
}
```

Note: `brace-expansion` may need nested overrides for 5.x instances (`>=5.0.7`) if pnpm keeps a 5.0.5 — verify with `pnpm why brace-expansion`.

### `edgequake-website`

```json
{
  "postcss": ">=8.5.12",
  "sharp": ">=0.35.0",
  "js-yaml": ">=4.3.0",
  "fast-uri": ">=3.1.4",
  "svgo": ">=4.0.2",
  "picomatch": ">=4.0.4",
  "smol-toml": ">=1.6.1",
  "yaml": ">=2.8.3",
  "esbuild": ">=0.28.1",
  "@babel/core": ">=7.29.6",
  "vite": ">=6.4.3"
}
```

After Astro 7, drop/adjust `vite` override if Astro pins Vite 8 securely.

### `mcp` (`overrides`)

```json
{
  "hono": ">=4.12.27",
  "@hono/node-server": ">=2.0.5",
  "fast-uri": ">=3.1.4",
  "body-parser": ">=2.3.0",
  "ip-address": ">=10.1.1",
  "rollup": ">=4.59.0",
  "esbuild": ">=0.28.1"
}
```

### `sdks/typescript`

```json
{
  "vite": ">=6.4.3",
  "postcss": ">=8.5.12",
  "brace-expansion": ">=2.1.2",
  "picomatch": ">=4.0.4",
  "minimatch": ">=9.0.7",
  "rollup": ">=4.59.0",
  "esbuild": ">=0.28.1"
}
```

---

## 3. Upgrade procedure (all surfaces)

1. Merge override JSON into package manifests (preserve existing keys).  
2. `pnpm install` / `npm install`.  
3. Prove each floor: `pnpm why <pkg>` / `npm ls <pkg> --all`.  
4. Run surface tests (see [04-verification-matrix.md](../04-verification-matrix.md)).  
5. If an override does not stick (peer dependency hard pin), bump the **parent** package instead — document PARTIAL in register.

---

## 4. minimatch special note

WebUI currently has `"minimatch": ">=3.1.5"`. Dependabot high alert floor is **9.0.7** for the affected range. **Raise** the override; re-run eslint/tooling tests (minimatch majors are commonly nested under eslint).

---

## 5. brace-expansion line-aware note

| Instance major | Floor |
|----------------|-------|
| 2.x | ≥2.1.2 |
| 3.x–5.x | ≥5.0.7 |

Use `pnpm.overrides` with package name only first; if mixed majors remain vulnerable, add selective `package@version` overrides or bump `minimatch` parents.

---

## 6. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_form_data_406` | form-data ≥4.0.6 |
| `sec085_fast_uri_314` | fast-uri ≥3.1.4 |
| `sec085_body_parser_230` | body-parser ≥2.3.0 |
| `sec085_ip_address_1011` | ip-address ≥10.1.1 |
| `sec085_transitive_sweep` | no open alerts for packages in §1 |

---

## 7. Cross-refs

Waves 3 + 6 · [05-surface-playbooks.md](../05-surface-playbooks.md) · [01-alert-register.md](../01-alert-register.md)
