---
title: "Upgrade to EdgeQuake v0.26.4"
description: "Per-release upgrade notes for EdgeQuake v0.26.4: what changed and what to run."
---

# Upgrade to EdgeQuake v0.26.4

> **From:** v0.26.3 · **To:** v0.26.4 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Ops/product patch: SPEC-144 Next.js **16.3.3** Active LTS (August 2026
Critical RCEs) + proxy SSOT + webpack Docker parity; SPEC-140/141 list
completeness; SPEC-122 bulk-ingest honesty; WebUI health poll off by default;
distroless API runtime. **No new migrations** — schema train remains **149**
from [upgrade-to-0.26.0.md](upgrade-to-0.26.0.md).

## Highlights

| Area | What changed |
|------|----------------|
| Next.js | `next` / `eslint-config-next` **16.3.3** (was 16.2.11) — August Critical GHSAs |
| Proxy | Single `src/proxy.ts` owns auth + swagger (root `middleware.ts` removed) |
| Docker WebUI | `next build --webpack` (parity with local safe-build) |
| Lists | SPEC-140/141 — honest `total` + catalog exhaustion / documents pager |
| Health poll | Off by default; set `EDGEQUAKE_HEALTH_POLL_MS=10000` to restore |
| API image | Distroless — no `curl`/`sh` in the container; use `edgequake healthcheck` |

Instant Navigations (`cacheComponents` / `partialPrefetching`) remain **off**
(documented React postpone blocker on webpack prerender). Free 16.3 wins still
apply without those flags.

## Sequence

```text
1. Pull GHCR images for 0.26.4 (especially edgequake-frontend — Next 16.3.3)
2. Deploy v0.26.4 API + frontend (no migrate step — schema still 149)
3. Verify /health and OpenAPI versions are 0.26.4
4. Confirm WebUI boots (Next 16.3.3 image)
```

If you still have leftover SPEC-091 DROP OLD (125/126/131) mid-cutover, follow
[upgrade-to-0.26.3.md](upgrade-to-0.26.3.md) / [upgrade-to-0.26.0.md](upgrade-to-0.26.0.md)
with the 0.26.4 API image (engine fixes from 0.26.3 are included).

### Distroless API note

Do **not** `docker exec … curl` inside the API container — there is no shell
or curl. Probe from outside:

```bash
curl -s http://localhost:8080/health
# or, as the container healthcheck does:
# edgequake healthcheck   # GET /live
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.26.4 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.26.4 make k8s-install
# or set global.edgequakeVersion: "0.26.4" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.4
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.4
# WebUI: open http://localhost:3000 — dashboard chrome visible
```

## Out of scope in this cut

- New schema / migrate step (train stays **149**)
- Fresh Acc n=200 medical-mid run
- Instant Navigations flags on
- Turbopack production / NFT retry

Detail: [`specs/144-update-nextjs/`](../../specs/144-update-nextjs/) ·
August Next security: [next@16.3.3](https://github.com/vercel/next.js/releases/tag/v16.3.3).
