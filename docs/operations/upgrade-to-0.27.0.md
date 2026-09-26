# Upgrade to EdgeQuake v0.27.0

> **From:** v0.26.x · **To:** v0.27.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Minor cut: **SPEC-149** provider-access / P0 projection authority + **SPEC-150**
reliable migration lifecycle. Schema train moves **149 → 159**. Serve no longer
applies migrations; operators must run `edgequake migrate` (or the compose /
Helm migrate Job) before `/ready` is 200.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Migrations **150–159** (provider-access ledger → `migration_run` telemetry) |
| Migrate ≠ serve | `edgequake migrate` owns DDL/support reconcile; serve is verify-only |
| Schema gate | `EDGEQUAKE_SCHEMA_GATE=wait\|fail` (compose/Helm default **wait**) |
| Exit codes | Lock busy → **75**; unknown fossil checksum → **65**; pending schema → **78** (fail mode) |
| Fossils | Manifest fossils auto-accept on migrate; `dev_only` never auto |
| Projections | P0 graph/vector projection deliveries; stage `projecting` in WebUI |

## Sequence

```text
1. Pull GHCR images for 0.27.0
2. Run migrate one-shot BEFORE restarting API (compose migrate service / Helm pre-hook)
3. Deploy API + frontend with EDGEQUAKE_SCHEMA_GATE=wait
4. Smoke /live (200 during wait) then /ready (200 when ledger current)
5. Confirm health.version == 0.27.0 and schema.latest_version == 159
```

### Migrate before serve

```bash
# One-shot (compose overlay already wires this for GCP)
docker compose run --rm migrate

# Or on the VM install path
sudo /opt/edgequake/scripts/install-release.sh 0.27.0
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
EDGEQUAKE_VERSION=0.27.0 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.27.0 make k8s-install
# or set global.edgequakeVersion: "0.27.0" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.27.0
curl -s http://localhost:8080/health | jq '.schema.latest_version'  # expect 159
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.27.0
```

## Out of scope in this cut

- Closing #396 / SPEC-091 guard RED
- Fresh Acc n=200 medical-mid against schema 159 (attested existing `publish/latest` from 2026-08-15; same honesty pattern as 0.26.4/0.26.5)
- Nightly epoch-matrix image-mode first green (local FORCE_REPLAY 56/56 attested)

Detail: [`specs/150-reliable-migration-system/`](../../specs/150-reliable-migration-system/) ·
[`specs/149-data-access-improvements/`](../../specs/149-data-access-improvements/) ·
Prior pin: [upgrade-to-0.26.5.md](upgrade-to-0.26.5.md).
