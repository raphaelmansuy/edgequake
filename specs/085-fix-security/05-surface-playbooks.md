# SPEC-085 — Surface Playbooks (DRY)

> **Status**: Active  
> **Laws**: LAW-16, LAW-17, LAW-19, LAW-20  
> **Rule**: Run only the playbook for the surface you touch. Never hand-edit lockfiles.

---

## 0. Shared rules

1. Bump **declared** versions in manifests; regenerate locks with the package manager.  
2. For **transitives**, prefer `pnpm.overrides` / npm `overrides` to the **security floor**.  
3. After install: prove resolved version (`pnpm why` / `npm ls` / `cargo tree` / `mvn dependency:tree`).  
4. Run the surface gate from [04-verification-matrix.md](04-verification-matrix.md).  
5. Confirm Dependabot alerts for that package transition to fixed (or re-query `gh api`).

---

## 1. `edgequake_webui` (pnpm)

```bash
cd edgequake_webui

# Direct floors (Wave 0–1)
pnpm add next@16.2.11
pnpm add axios@^1.18.1
pnpm add dompurify@^3.4.12

# Align eslint-config-next with next when bumping
pnpm add -D eslint-config-next@16.2.11

# Transitive floors — extend existing pnpm.overrides (do not replace lodash/follow-redirects/rollup/minimatch)
# Example package.json pnpm.overrides additions:
#   "postcss": ">=8.5.12",
#   "sharp": ">=0.35.0",
#   "js-yaml": ">=4.3.0",
#   "brace-expansion": ">=2.1.2",
#   "form-data": ">=4.0.6",
#   "flatted": ">=3.4.2",
#   "vite": ">=7.3.5",
#   "@babel/core": ">=7.29.6",
#   "esbuild": ">=0.28.1"

pnpm install
pnpm why next axios dompurify postcss sharp
pnpm run typecheck && pnpm test && pnpm run build
```

**SSOT**: `pnpm-lock.yaml`. Do not trust `bun.lock`.

---

## 2. `edgequake-website` (pnpm)

```bash
cd edgequake-website

# Wave 2 — major (required by LAW-15 Astro floor ≥7.1.0)
pnpm dlx @astrojs/upgrade
# or: pnpm add astro@^7.1.3  # plus official integrations together

# After Astro 7: expect Vite 8 line; still enforce floors via overrides if needed
#   "svgo": ">=4.0.2",
#   "sharp": ">=0.35.0",
#   "js-yaml": ">=4.3.0",
#   "fast-uri": ">=3.1.4",
#   "postcss": ">=8.5.12",
#   "picomatch": ">=4.0.4",
#   "smol-toml": ">=1.6.1",
#   "yaml": ">=2.8.3",
#   "esbuild": ">=0.28.1",
#   "@babel/core": ">=7.29.6"

pnpm install
pnpm run build
```

Migration checklist: [Astro v7 upgrade guide](https://docs.astro.build/en/guides/upgrade-to/v7/) — Rust compiler strict HTML, `compressHTML: 'jsx'` default, reserved `src/fetch.ts`, Vite 8.

---

## 3. `mcp` (npm)

```bash
cd mcp

# Prefer bumping @modelcontextprotocol/sdk if it pulls patched hono;
# otherwise npm overrides:
#   "overrides": {
#     "hono": ">=4.12.27",
#     "@hono/node-server": ">=2.0.5",
#     "fast-uri": ">=3.1.4",
#     "body-parser": ">=2.3.0",
#     "ip-address": ">=10.1.1",
#     "rollup": ">=4.59.0",
#     "esbuild": ">=0.28.1"
#   }

npm install
npm ls hono @hono/node-server fast-uri
npm test
```

---

## 4. `sdks/typescript` (npm lockfile)

```bash
cd sdks/typescript

# Wave 0 — Critical
npm install -D vitest@^3.2.6
# If webui uses vitest 4.x, evaluate line-specific patched floor separately;
# Dependabot Critical alert is on this lockfile at <3.2.6.

# Overrides for vite line in this tree (currently vite 6.x):
#   "vite": ">=6.4.3",
#   "postcss": ">=8.5.12",
#   "brace-expansion": ">=2.1.2",
#   "picomatch": ">=4.0.4",
#   "minimatch": ">=9.0.7",
#   "rollup": ">=4.59.0",
#   "esbuild": ">=0.28.1"

npm install
npm test
npm ls vitest vite
```

---

## 5. Maven SDKs (Java + Kotlin)

```bash
# LAW-16: one property, both POMs
# sdks/java/pom.xml  and  sdks/kotlin/pom.xml
#   <jackson.version>2.18.9</jackson.version>   # 2.18.9 is floor AND latest 2.18.x (2026-07-24)

cd sdks/java && mvn -q test
cd sdks/kotlin && mvn -q test
```

---

## 6. Rust workspace (`edgequake/`)

```bash
cd edgequake

# LAW-21: remove jsonwebtoken = "9.3" from workspace + edgequake-api;
# keep edgequake-auth as SSOT: jsonwebtoken = { version = "10.3", features = ["aws_lc_rs"] }
# (resolved 10.4.0 is fine)

# OTEL coordinated bump (observability crate):
#   opentelemetry = "0.32"
#   opentelemetry_sdk = "0.32"          # floor ≥0.32.1
#   opentelemetry-otlp = "0.32"
#   tracing-opentelemetry = compatible

cargo update -p jsonwebtoken
cargo update -p opentelemetry_sdk
cargo test -p edgequake-auth
cargo test -p edgequake-observability --features otel   # adjust feature name to crate reality
cargo tree -p jsonwebtoken -i
```

---

## 7. Rust SDK (`sdks/rust`)

```bash
cd sdks/rust
cargo update -p aws-lc-sys
# Prove ≥0.39.0
cargo tree -p aws-lc-sys -i
cargo test
```

---

## 8. Global close-out (after Wave 6)

```bash
# From repo root
gh api repos/raphaelmansuy/edgequake/dependabot/alerts --paginate \
  --jq '[.[] | select(.state=="open")] | length'

# Expect 0 for packages listed in 01-alert-register.md (or only residual documented in PKG studies)
```
