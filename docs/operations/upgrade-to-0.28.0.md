---
title: "Upgrade to EdgeQuake v0.28.0"
description: "Per-release upgrade notes for EdgeQuake v0.28.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.0

> **From:** v0.27.0 · **To:** v0.28.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Minor cut: **SPEC-143** directional PDF/Markdown page sync + **SPEC-151**
partial page reprocess + interactive read-path hardening (#400) + community /
typed FTS storage fixes (#404/#405). Schema train moves **159 → 160**. Serve
still does not apply migrations; operators must run `edgequake migrate` (or
the compose / Helm migrate Job) before `/ready` is 200.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Migration **160** (`document_page_states` for partial page reprocess) |
| SPEC-143 | Side-by-side sync modes: `none`, `pdf-to-md` (default), `md-to-pdf` |
| SPEC-151 | `GET …/pages/health`, `POST …/pages/reprocess` (`dry_run` previews) |
| #400 | Catalog lists return 503 `read_path_busy` under load; WebUI retries once |
| #404 | Community refresh uses keyset scans + statement timeout; size gates skip |
| #405 | Default typed FTS/ANN never SELECTs retired `eq_*_vectors` |

## Sequence

```text
1. Pull GHCR images for 0.28.0
2. Run migrate one-shot BEFORE restarting API (compose migrate service / Helm pre-hook)
3. Deploy API + frontend with EDGEQUAKE_SCHEMA_GATE=wait
4. Smoke /live (200 during wait) then /ready (200 when ledger current)
5. Confirm health.version == 0.28.0 and schema.latest_version == 160
```

### Migrate before serve

```bash
# One-shot (compose overlay already wires this for GCP)
docker compose run --rm migrate

# Or on the VM install path
sudo /opt/edgequake/scripts/install-release.sh 0.28.0
```

Do **not** rely on API boot to apply pending migrations. Escape hatch only:
`EDGEQUAKE_SERVE_RECONCILE=1` (one-release; not for ongoing ops).

### Distroless API note

Do **not** `docker exec … curl` inside the API container. Probe from outside:

```bash
curl -sS https://demo.edgequake.com/live
curl -sS https://demo.edgequake.com/ready
curl -sS http://localhost:8080/health | jq '{version, schema}'
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.28.0 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.28.0 make k8s-install
# or set global.edgequakeVersion: "0.28.0" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.28.0
curl -s http://localhost:8080/health | jq '.schema.latest_version'  # expect 160
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.28.0
```

## Out of scope in this cut

- Fresh Acc n=200 medical-mid against schema 160 (attested existing `publish/latest` from 2026-08-15; same honesty pattern as 0.27.0)
- Helm chart default pin remains `0.26.4` (override with `EDGEQUAKE_VERSION` / `global.edgequakeVersion`)

Detail: [`specs/151-partial-preprocess/`](../../specs/151-partial-preprocess/) ·
[`specs/143-view-pdf-markdown-sync-view/`](../../specs/143-view-pdf-markdown-sync-view/) ·
Prior pin: [upgrade-to-0.27.0.md](upgrade-to-0.27.0.md).
