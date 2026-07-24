# SPEC-085 — Cross-Reference Matrix

> Package ↔ manifests ↔ laws ↔ wave ↔ verification IDs

| Package | Primary manifests | Laws | Wave | Verify IDs |
|---------|-------------------|------|------|------------|
| `vitest` | `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 0 | `sec085_vitest_floor` |
| `next` | `edgequake_webui/package.json`; `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 0 | `sec085_next_16_2_11` |
| `com.fasterxml.jackson.core:jackson-databind` | `sdks/java/pom.xml`; `sdks/kotlin/pom.xml` | 15,16,20 | 4 | `sec085_jackson_2189` |
| `vite` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_vite_line` |
| `axios` | `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 1 | `sec085_axios_118` |
| `astro` | `edgequake-website/package.json`; `edgequake-website/pnpm-lock.yaml` | 15,16,17,18,20 | 2 | `sec085_astro_71` |
| `hono` | `mcp/package-lock.json` | 15,16,17,19,20 | 3 | `sec085_hono_41227` |
| `fast-uri` | `edgequake-website/pnpm-lock.yaml`; `mcp/package-lock.json` | 15,16,17,19,20 | 3 | `sec085_fast_uri_314` |
| `postcss` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_postcss_8512` |
| `aws-lc-sys` | `sdks/rust/Cargo.lock` | 15,16,17,19,20 | 5 | `sec085_aws_lc_039` |
| `js-yaml` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_js_yaml_430` |
| `brace-expansion` | `edgequake_webui/pnpm-lock.yaml`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_transitive_brace-expansion` |
| `picomatch` | `edgequake-website/pnpm-lock.yaml`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_transitive_picomatch` |
| `minimatch` | `edgequake_webui/pnpm-lock.yaml`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_transitive_minimatch` |
| `rollup` | `mcp/package-lock.json`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_transitive_rollup` |
| `sharp` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_sharp_035` |
| `flatted` | `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_transitive_flatted` |
| `form-data` | `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 1 | `sec085_form_data_406` |
| `svgo` | `edgequake-website/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_transitive_svgo` |
| `dompurify` | `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 1 | `sec085_dompurify_3412` |
| `jsonwebtoken` | `edgequake/Cargo.lock`; `edgequake/crates/edgequake-api/Cargo.toml` | 15,16,21,20 | 5 | `sec085_jwt_103` |
| `opentelemetry_sdk` | `edgequake/Cargo.lock`; `edgequake/crates/edgequake-observability/Cargo.toml` | 15,16,17,19,20 | 5 | `sec085_otel_0321` |
| `@hono/node-server` | `mcp/package-lock.json` | 15,16,17,19,20 | 3 | `sec085_hono_41227` |
| `ip-address` | `mcp/package-lock.json` | 15,16,17,19,20 | 3 | `sec085_ip_address_1011` |
| `smol-toml` | `edgequake-website/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_transitive_smol-toml` |
| `yaml` | `edgequake-website/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_transitive_yaml` |
| `esbuild` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml`; `mcp/package-lock.json`; `sdks/typescript/package-lock.json` | 15,16,17,19,20 | 6 | `sec085_transitive_esbuild` |
| `@babel/core` | `edgequake-website/pnpm-lock.yaml`; `edgequake_webui/pnpm-lock.yaml` | 15,16,17,19,20 | 6 | `sec085_transitive_babel_core` |
| `body-parser` | `mcp/package-lock.json` | 15,16,17,19,20 | 3 | `sec085_body_parser_230` |

---

## Dependency graph

```
  vitest --------------------> vite --> esbuild / rollup / postcss
  next ----------------------> sharp (image opt)
  axios ---------------------> form-data
  MCP SDK -------------------> hono / @hono/node-server
  ajv -----------------------> fast-uri
  minimatch -----------------> brace-expansion
  astro ---------------------> vite / svgo / sharp / js-yaml
  jsonwebtoken(aws_lc_rs) --> aws-lc-sys (rust-sdk / auth)
  edgequake-observability --> opentelemetry_sdk
  jackson.version -----------> jackson-databind (java+kotlin)
```

## Explicit non-dependencies

| Claim | Reality |
|-------|--------|
| Next 16.3 canary required for July 2026 CVEs | **False** — 16.2.11 is Active LTS floor |
| Astro 6.4.6 closes all website Astro GHSAs | **False** — GHSA-4g3v needs ≥7.1.0 |
| Bumping only edgequake-auth jsonwebtoken closes alerts | **False** — workspace/API still pin 9.3 |
| Hand-editing pnpm-lock closes alerts safely | **False** — LAW-19 |
| bun.lock is webui SSOT | **False** — pnpm-lock.yaml |
