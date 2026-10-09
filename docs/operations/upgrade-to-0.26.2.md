---
title: "Upgrade to EdgeQuake v0.26.2"
description: "Per-release upgrade notes for EdgeQuake v0.26.2: what changed and what to run."
---

# Upgrade to EdgeQuake v0.26.2

> **From:** v0.26.1 · **To:** v0.26.2 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Ops/product patch: Langfuse 3.1.x ingestion fallback (SPEC-124), Kubernetes
Helm/kind (SPEC-138), SSE/conversation restore, workspace `include_stats`.
**No new migrations** — schema train remains **149** from
[upgrade-to-0.26.0.md](upgrade-to-0.26.0.md).

## Highlights

| Area | What changed |
|------|----------------|
| Langfuse 3.1.x | `EDGEQUAKE_LANGFUSE_API=auto` probes OTLP once; HTTP 404 → native `/api/public/ingestion` |
| Kubernetes | Helm `edgequake-stack` + kind proof; pin `global.edgequakeVersion: "0.26.2"` |
| SSE | `text/event-stream` is not gzip-compressed; conversation identity is shared |
| Workspace list | Opt-in `?include_stats=true` (default off) |

This cut does **not** add schema. Leftover DROP OLD (125 / 126 / 131) still
follows the SPEC-091 ladder in [upgrade-to-0.26.0.md](upgrade-to-0.26.0.md)
using a **0.26.1+** binary ([upgrade-to-0.26.1.md](upgrade-to-0.26.1.md) CLI
honesty).

## Sequence

```text
1. Backup (optional for this patch — no schema train)
2. Deploy v0.26.2 API (and frontend if you pin it)
3. If leftover 125 / 126 / 131 remain, follow upgrade-to-0.26.0.md with this
   0.26.2 binary (not 0.26.0)
4. Verify health version is 0.26.2
5. If you point at self-hosted Langfuse 3.1.x, confirm api_resolved=ingestion
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.26.2 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.26.2 make k8s-install
# or set global.edgequakeVersion: "0.26.2" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.2
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.2
```

Langfuse 3.1.x only (keys + `LANGFUSE_BASE_URL` of *that* instance):

```bash
curl -sS http://localhost:8080/api/v1/settings/langfuse \
  | jq '{export_active, base_url, api, api_resolved}'
# api_resolved must be "ingestion" on 3.1.x (never force EDGEQUAKE_LANGFUSE_API=otlp)
```

Operator guide: [langfuse-3.1.md](langfuse-3.1.md) · Kubernetes:
[deploy/kubernetes/README.md](../../deploy/kubernetes/README.md#existing-langfuse-31x).

## Out of scope in this cut

- New schema / migrate step (train stays **149**)
- Fresh Acc n=200 medical-mid run (attested existing `publish/latest`)
- crates.io publish of EdgeQuake workspace crates (GHCR-only CD)
- Forcing OTLP against Langfuse 3.1.x (use `auto`; upgrade Langfuse ≥ 3.22 / v4)

Detail: [`specs/124-langfuse-support/`](../../specs/124-langfuse-support/) ·
[`specs/138-kubernetes/`](../../specs/138-kubernetes/).
