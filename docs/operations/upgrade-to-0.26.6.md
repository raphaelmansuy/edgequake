# Upgrade to EdgeQuake v0.26.6

> **From:** v0.26.5 · **To:** v0.26.6 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Ops/product patch: Vision PDF figure pipeline skips encoding-artifact pages
(SPEC-147); graph cascade delete discovery uses index-friendly singular
citation probes (SPEC-119); Vision stall timeouts remain retryable until the
circuit breaker trips so checkpoint resume works. Also includes #396 embedding
migration residual hardening and related reliability fixes.
**No new migrations** — schema train remains **149** from
[upgrade-to-0.26.0.md](upgrade-to-0.26.0.md).

## Highlights

| Area | What changed |
|------|----------------|
| Vision figures | Page-level inventory + PDF subset; storm/OCR-tile pages never decoded for Pass-B |
| Graph delete | Singular `source_chunk_id` / `source_document_id` probes use `= ANY` Index Scan |
| Vision stall | Timeout factory stays retryable until breaker trips (checkpoint resume) |
| Embed migrate | #396 DISTINCT ON arbiter + residual 21000/23505 savepoint path |

Restart the **API** after deploy so Vision figure skip and delete discovery
fixes load (frontend image unchanged for this cut unless you pull `latest`).

## Sequence

```text
1. Pull GHCR images for 0.26.6 (especially edgequake API)
2. Deploy v0.26.6 API + frontend (no migrate step — schema still 149)
3. Verify /health and OpenAPI versions are 0.26.6
4. Optional: retry a previously stuck delete_failed doc; reprocess a Vision PDF that stalled mid-convert
```

### Distroless API note

Do **not** `docker exec … curl` inside the API container — there is no shell
or curl. Probe from outside:

```bash
curl -s http://localhost:8080/health
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.26.6 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.26.6 make k8s-install
# or set global.edgequakeVersion: "0.26.6" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.6
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.6
```

## Out of scope in this cut

- New schema / migrate step (train stays **149**)
- Fresh Acc n=200 medical-mid run (attested existing `publish/latest`)
- crates.io publish of workspace crates (GHCR-only)

Detail: [`CHANGELOG.md`](../../CHANGELOG.md) · runbook:
[`release-and-cd.md`](release-and-cd.md).
