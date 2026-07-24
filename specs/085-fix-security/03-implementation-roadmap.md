# SPEC-085 — Implementation Roadmap

> **Status**: Implemented 2026-07-24 (otel residual cleared via pdf2md 0.9.8)  
> **Laws**: LAW-15…LAW-21  
> **Cross-refs**: [Register](01-alert-register.md) · [Matrix](02-cross-ref-matrix.md) · [Verification](04-verification-matrix.md) · [Playbooks](05-surface-playbooks.md)

---

## Wave graph

```
  Wave0 CriticalRuntime ──┬──> Wave1 WebUI Direct ──> Wave2 Website Astro7 ──┐
                          │                                                   │
                          ├──> Wave3 MCP Stack ───────────────────────────────┼──> Wave6 Transitive Sweep
                          │                                                   │
                          ├──> Wave4 Maven Jackson ───────────────────────────┤
                          │                                                   │
                          └──> Wave5 Rust Auth/OTEL/aws-lc ───────────────────┘
```

Waves 1/3/4/5 may proceed in parallel after Wave 0 lands on their surfaces. Wave 2 is isolated (major). Wave 6 is the final override sweep after direct bumps settle.

---

## Wave 0 — Critical + runtime high

| Package | Floor | Action |
|---------|-------|--------|
| `vitest` | ≥3.2.6 | Bump `sdks/typescript` (Critical GHSA). Re-evaluate webui `vitest@^4.1.0` for line-specific advisories. |
| `next` | ≥16.2.11 | Pin `edgequake_webui` to `16.2.11`; align `eslint-config-next`. |

**Exit**: `sec085_vitest_floor` + `sec085_next_16_2_11` green; Dependabot Critical/Next high alerts fixed.

**Study**: [PKG-vitest](packages/PKG-vitest.md) · [PKG-next](packages/PKG-next.md)

---

## Wave 1 — WebUI direct deps

| Package | Floor | Action |
|---------|-------|--------|
| `axios` | ≥1.18.0 (prefer 1.18.1) | Direct bump in webui |
| `dompurify` | ≥3.4.12 | Direct bump in webui |
| `form-data` | ≥4.0.6 | Follows axios or override |

**Exit**: upload/query smoke + unit tests; XSS sanitizer paths still pass.

**Study**: [PKG-axios](packages/PKG-axios.md) · [PKG-dompurify](packages/PKG-dompurify.md)

---

## Wave 2 — Website Astro 7 (major)

| Package | Floor | Action |
|---------|-------|--------|
| `astro` | ≥7.1.0 (prefer latest 7.1.x) | `pnpm dlx @astrojs/upgrade` + v7 checklist |

**Pulls**: vite line, svgo, sharp, js-yaml (may clear or reduce Wave 6 work for website).

**Exit**: production build; visual spot-check; `compressHTML` decision recorded.

**Study**: [PKG-astro](packages/PKG-astro.md)

---

## Wave 3 — MCP stack

| Package | Floor | Action |
|---------|-------|--------|
| `hono` | ≥4.12.27 | Override or bump MCP SDK parent |
| `@hono/node-server` | ≥2.0.5 | Same |
| `fast-uri` | ≥3.1.4 | Override |
| `body-parser` | ≥2.3.0 | Override |
| `ip-address` | ≥10.1.1 | Override |

**Exit**: `npm test` in `mcp/`; `npm ls` proves floors.

**Study**: [PKG-hono-stack](packages/PKG-hono-stack.md) · [PKG-transitive-npm](packages/PKG-transitive-npm.md)

---

## Wave 4 — Maven Jackson

| Package | Floor | Action |
|---------|-------|--------|
| `jackson-databind` | ≥2.18.9 | `${jackson.version}` in java + kotlin POMs |

**Exit**: `mvn test` both SDKs.

**Study**: [PKG-jackson-databind](packages/PKG-jackson-databind.md)

---

## Wave 5 — Rust

| Package | Floor | Action |
|---------|-------|--------|
| `jsonwebtoken` | ≥10.3.0 | LAW-21: remove 9.3 pins; auth SSOT |
| `opentelemetry_sdk` | ≥0.32.1 | Coordinated OTEL stack bump |
| `aws-lc-sys` | ≥0.39.0 | `cargo update` in `sdks/rust` (+ workspace if pulled) |

**Exit**: auth tests + observability feature tests + rust SDK build.

**Study**: [PKG-jsonwebtoken](packages/PKG-jsonwebtoken.md) · [PKG-opentelemetry-sdk](packages/PKG-opentelemetry-sdk.md) · [PKG-aws-lc-sys](packages/PKG-aws-lc-sys.md)

---

## Wave 6 — Transitive sweep

Packages: `vite`, `postcss`, `sharp`, `js-yaml`, `brace-expansion`, `picomatch`, `minimatch`, `rollup`, `flatted`, `svgo`, `esbuild`, `@babel/core`, `smol-toml`, `yaml`, plus any residual from Waves 2–3.

**Action**: surface overrides (playbooks) + lock regenerate; prove with `pnpm why` / `npm ls`.

**Exit**: open Dependabot count for listed packages → 0 (or residual documented in PKG study).

**Study**: [PKG-vite](packages/PKG-vite.md) · [PKG-postcss](packages/PKG-postcss.md) · [PKG-sharp](packages/PKG-sharp.md) · [PKG-js-yaml](packages/PKG-js-yaml.md) · [PKG-transitive-npm](packages/PKG-transitive-npm.md)

---

## PR slicing (recommended)

| PR | Wave | Surfaces |
|----|------|----------|
| A | 0 | webui next + ts-sdk vitest |
| B | 1 | webui axios/dompurify |
| C | 2 | website astro 7 |
| D | 3 | mcp overrides |
| E | 4 | java+kotlin jackson |
| F | 5 | rust jwt/otel/aws-lc |
| G | 6 | transitive overrides all npm surfaces |

Do **not** mix Astro major with Next patch in one PR (LAW-17).

---

## Definition of done (pack execution)

1. Register status column → FIXED for all 29 packages (or RETRACTED with residual note).  
2. `gh api .../dependabot/alerts` open count = 0 for in-scope packages.  
3. All verification IDs in [04-verification-matrix.md](04-verification-matrix.md) checked.  
4. No dual `jsonwebtoken` majors in the workspace.
