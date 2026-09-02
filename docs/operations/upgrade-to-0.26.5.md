# Upgrade to EdgeQuake v0.26.5

> **From:** v0.26.4 · **To:** v0.26.5 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Ops/product patch: **SPEC-145** Langfuse generation / LLM observation I/O is
complete by default (full prompt + completion; no product-side length
truncation). Secrets remain redacted; ingest document **content** stays Preview.
**No new migrations** — schema train remains **149** from
[upgrade-to-0.26.0.md](upgrade-to-0.26.0.md).

## Highlights

| Area | What changed |
|------|----------------|
| Langfuse I/O | Generation Input/Output = full LLM payload (Complete class) |
| Default ceiling | `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES=0` (unlimited); optional positive clamp |
| Stream | Generation span held until tokens end; I/O recorded once assembled |
| Helm | `api.langfuse.ioMaxBytes` → ConfigMap |

Restart the **API** after deploy so new traces pick up Complete I/O (frontend
image unchanged for this cut).

## Sequence

```text
1. Pull GHCR images for 0.26.5 (especially edgequake API)
2. Deploy v0.26.5 API + frontend (no migrate step — schema still 149)
3. Verify /health and OpenAPI versions are 0.26.5
4. Run a long Mix/query and confirm Langfuse generation Input/Output are complete
```

### Distroless API note

Do **not** `docker exec … curl` inside the API container — there is no shell
or curl. Probe from outside:

```bash
curl -s http://localhost:8080/health
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.26.5 docker compose -f docker-compose.quickstart.yml up -d
```

Kubernetes:

```bash
EDGEQUAKE_VERSION=0.26.5 make k8s-install
# or set global.edgequakeVersion: "0.26.5" in values
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.5
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.5
# Optional: make spec145-langfuse-e2e (live OTLP + tail marker)
```

## Out of scope in this cut

- New schema / migrate step (train stays **149**)
- Fresh Acc n=200 medical-mid run (attested existing `publish/latest`)
- Dumping ingest document bodies into Langfuse

Detail: [`specs/145-fix-truncated-logs/`](../../specs/145-fix-truncated-logs/) ·
Observability: [`docs/OBSERVABILITY.md`](../OBSERVABILITY.md).
