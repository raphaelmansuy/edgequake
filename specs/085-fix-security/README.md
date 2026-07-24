# SPEC-085 — Dependabot Security Upgrade Pack

> **Product pin**: EdgeQuake v0.21.1  
> **Source**: [Dependabot alerts](https://github.com/raphaelmansuy/edgequake/security/dependabot) (`gh api .../dependabot/alerts`)  
> **Docs status**: Implementation landed 2026-07-24  
> **Audit date**: 2026-07-24  
> **Inventory at audit**: 133 open alerts · 29 unique packages · ~88 unique GHSAs

## Verification status (SSOT)

See [01-alert-register.md](01-alert-register.md): **29 FIXED / 0 PARTIAL / 0 OPEN**.

| Wave | Goal | Status |
|------|------|--------|
| **0** | Critical + runtime high (`vitest`, `next@16.2.11`) | **done** |
| **1** | WebUI direct (`axios`, `dompurify` → `form-data`) | **done** |
| **2** | Website major (`astro≥7.1`) | **done** |
| **3** | MCP stack (`hono`, `@hono/node-server`, `fast-uri`, …) | **done** |
| **4** | Maven Jackson (`jackson-databind≥2.18.9`) | **done** |
| **5** | Rust (`jsonwebtoken` SSOT, `opentelemetry_sdk`, `aws-lc-sys`) | **done** |
| **6** | Transitive sweep (overrides + lock regen) | **done** |

**Closed residual (2026-07-24)**: `edgequake-pdf2md@0.9.8` (crates.io) pins `edgequake-llm@0.10.2`; lock has only `opentelemetry_sdk@0.32.1`.

---

## Start here

1. Read [00-first-principles.md](00-first-principles.md) — LAW-15…LAW-21 + SOLID/DRY  
2. Skim [01-alert-register.md](01-alert-register.md) — every package with floor + wave  
3. Cross-refs → [02-cross-ref-matrix.md](02-cross-ref-matrix.md)  
4. Roadmap → [03-implementation-roadmap.md](03-implementation-roadmap.md)  
5. Gates → [04-verification-matrix.md](04-verification-matrix.md)  
6. Commands → [05-surface-playbooks.md](05-surface-playbooks.md)  
7. Floors vs latest → [06-version-pins.md](06-version-pins.md)  
8. Package studies → [`packages/`](packages/README.md)

---

## Locked decisions

1. **Next** stays on **16.2.x LTS patch** (`16.2.11+`), not canary `16.3`.  
2. **Astro** upgrades to **≥7.1.x** (floor for remaining XSS); no “stay on 6.x”.  
3. **Jackson** bumps `${jackson.version}` once in Java + Kotlin POMs.  
4. **`jsonwebtoken`**: remove workspace / `edgequake-api` **9.3** pin; auth crate SSOT ≥10.3 + `aws_lc_rs`.  
5. Extend existing **pnpm overrides** (webui pattern) for transitive floors — do not invent a second mechanism.  
6. **`edgequake_webui/pnpm-lock.yaml` is SSOT**; ignore stale `bun.lock`.  
7. **WebUI production build** uses `next build --webpack` (Turbopack + `output: "standalone"` hit `middleware.js.nft.json` ENOENT on 16.2.11).

---

## E2E proof (2026-07-24)

| Check | Result |
|-------|--------|
| `GET :8090/health` | `healthy` / postgresql / v0.21.1 |
| `GET :8090/live` | OK |
| Next.js version | **16.2.11** (dev on :3010) |
| `GET :3010/` / documents | HTTP 200 |
| `next build --webpack` | success (standalone) |
| Astro `pnpm build` | 100 pages |
| MCP / TS SDK / Maven / auth / rust-sdk tests | pass |

---

## Surfaces (blast radius)

| Surface | Manager | Lock / resolve |
|---------|---------|----------------|
| `edgequake_webui` | pnpm | `pnpm-lock.yaml` |
| `edgequake-website` | pnpm | `pnpm-lock.yaml` |
| `mcp` | npm | `package-lock.json` |
| `sdks/typescript` | npm lock | `package-lock.json` |
| `sdks/java`, `sdks/kotlin` | Maven | `pom.xml` property |
| Rust workspace | cargo | `edgequake/Cargo.lock` |
| `sdks/rust` | cargo | `sdks/rust/Cargo.lock` |
