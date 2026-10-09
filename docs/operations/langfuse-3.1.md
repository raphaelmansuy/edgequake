---
title: EdgeQuake + Langfuse 3.1.x
description: "Configure Langfuse 3.1 with EdgeQuake."
---

# EdgeQuake + Langfuse 3.1.x

How to send EdgeQuake RAG traces to a **self-hosted Langfuse 3.1.x** instance (including **3.1.1**).

Langfuse added OTLP (`POST /api/public/otel/v1/traces`) in **[v3.22.0](https://github.com/langfuse/langfuse/releases/tag/v3.22.0)**. **3.1.x has no OTLP path** — it returns HTTP 404. EdgeQuake’s default `EDGEQUAKE_LANGFUSE_API=auto` probes that path once at startup and, **only on 404**, exports via the legacy native API `POST /api/public/ingestion`.

```ascii
  EdgeQuake API (otel feature on)
       │
       ├─ LANGFUSE_PUBLIC_KEY + LANGFUSE_SECRET_KEY  (required)
       └─ LANGFUSE_BASE_URL = your 3.1 host          (required — empty falls back to Cloud)
              │
              EDGEQUAKE_LANGFUSE_API=auto  (default)
              │
              probe POST {base}/api/public/otel/v1/traces
              ├─ 404  → ingestion  POST {base}/api/public/ingestion
              └─ else → OTLP       (3.22+ / Cloud / v4)
```

This is a **compatibility bridge**. Langfuse Cloud sunsets trace events on the ingestion API on **2026-11-16**. Self-hosted **v4** `events_only` rejects it. Prefer upgrading Langfuse to **≥ 3.22** (or the in-repo v4 compose / Helm chart). Do not claim “any Langfuse ≥ 2.x” without this fallback.

Operator references: [OBSERVABILITY.md](../OBSERVABILITY.md) · [Kubernetes](../../deploy/kubernetes/README.md#existing-langfuse-31x) · SPEC-124.

---

## 1. Point EdgeQuake at *this* Langfuse

Create a project in **that** Langfuse instance and copy **its** keys. A `pk-lf-` / `sk-lf-` pair is only valid on the host that issued it.

| Variable | 3.1 requirement |
|----------|-----------------|
| `LANGFUSE_BASE_URL` | Origin of the 3.1 UI/API (no trailing path). **Never leave empty** — empty = Cloud. Alias: `LANGFUSE_HOST`. |
| `LANGFUSE_PUBLIC_KEY` | `pk-lf-…` from this instance |
| `LANGFUSE_SECRET_KEY` | `sk-lf-…` from this instance (never logged, never in the UI) |
| `LANGFUSE_PROJECT_ID` | Optional. Settings deep-link; otherwise fetched once from `GET /api/public/projects`. |
| `EDGEQUAKE_LANGFUSE_API` | **`auto`** (default). Use `ingestion` only if you must skip the probe. Do **not** set `otlp` against 3.1.x. |

Restart the API after changing these. `export_active: true` only means **keys are present**. Always check `base_url` and **`api_resolved`**.

---

## 2. Local: isolated Langfuse 3.1.1 (this repo)

Does **not** replace `make langfuse-up` (v4 on `:3310`). 3.1.1 is a separate compose project on **`:3320`**.

```bash
# Start Langfuse 3.1.1 (worker is restarted after web Prisma migrations)
make langfuse-3.1-up

# In repo-root .env (unquoted; make backend-bg sources it):
LANGFUSE_PUBLIC_KEY=pk-lf-edgequake-311
LANGFUSE_SECRET_KEY=sk-lf-edgequake-311-dev
LANGFUSE_BASE_URL=http://localhost:3320
LANGFUSE_PROJECT_ID=edgequake-local-311
# EDGEQUAKE_LANGFUSE_API=auto

make kill-app && make backend-bg   # or: make dev
```

| | Value |
|--|--------|
| UI | http://localhost:3320 |
| Headless keys | `pk-lf-edgequake-311` / `sk-lf-edgequake-311-dev` |
| Project id | `edgequake-local-311` |

Stop with `make langfuse-3.1-down` (volumes kept). Wipe with `make langfuse-3.1-reset CONFIRM=yes` (does **not** touch the v4 stack on `:3310`).

Unfakable proof (version pin + OTLP 404 + real exporter persist):

```bash
make spec124-langfuse-3.1-e2e
```

OTLP itself starts at **Langfuse 3.22.0**, not 3.2.x. Local pins + current Cloud:

```bash
make spec124-langfuse-3.22-e2e    # 3.22.0 UI :3330 — route exists + auto-probe=Otlp
make spec124-langfuse-3.225-e2e   # 3.225.5 UI :3340 — OTLP persist (current 3.x)
make spec124-langfuse-cloud-e2e   # current Cloud (LANGFUSE_* in .env)
make spec124-langfuse-matrix      # 3.1.1 + 3.22.0 + 3.225.5 + Cloud
```

3.22.0’s first-release OTLP protobuf parser raises `Invalid time value` on current OpenTelemetry timestamps. Persist is proven on **3.225.5** and Cloud, not on the 3.22.0 tag.

---

## 3. Existing self-hosted 3.1.x (Docker / VM / Kubernetes)

Same three secrets + base URL. Examples:

**Docker / systemd** — set on the EdgeQuake API process (not on Langfuse):

```bash
export LANGFUSE_BASE_URL=http://langfuse.internal.example:3000
export LANGFUSE_PUBLIC_KEY=pk-lf-...
export LANGFUSE_SECRET_KEY=sk-lf-...
export EDGEQUAKE_LANGFUSE_API=auto
```

Compose files in this repo (`docker-compose.yml`, quickstart, api-only, prebuilt) pass `EDGEQUAKE_LANGFUSE_API` through when set.

**Kubernetes / Helm** — in-cluster DNS, never `localhost` inside the API pod:

```yaml
# deploy/kubernetes/helm/edgequake/values.yaml  (api.langfuse)
api:
  langfuse:
    baseUrl: "http://langfuse-web.langfuse.svc.cluster.local:3000"  # your 3.1 Service
    projectId: "your-project-id"
    existingSecret: edgequake-langfuse-secret   # LANGFUSE_PUBLIC_KEY + LANGFUSE_SECRET_KEY
    api: auto   # Helm → EDGEQUAKE_LANGFUSE_API (ConfigMap)
```

Kind/Helm in this repo still **pin Langfuse v4** for SPEC-138. Pointing Helm at a **customer 3.1** chart is “bring your own Langfuse”: keep `api: auto`, use **that** instance’s keys, and confirm `api_resolved` is `ingestion`. Full Helm notes: [deploy/kubernetes/README.md](../../deploy/kubernetes/README.md#existing-langfuse-31x).

---

## 4. Verify (do not skip)

Confirm the instance is 3.1.x and has no OTLP:

```bash
curl -sS "$LANGFUSE_BASE_URL/api/public/health" | jq .version
# expect 3.1.x

curl -sS -o /dev/null -w '%{http_code}\n' \
  -X POST "$LANGFUSE_BASE_URL/api/public/otel/v1/traces" \
  -u "$LANGFUSE_PUBLIC_KEY:$LANGFUSE_SECRET_KEY" \
  -H 'Content-Type: application/x-protobuf'
# expect 404
```

Confirm EdgeQuake resolved **ingestion** (after API restart):

```bash
curl -sS http://localhost:8080/api/v1/settings/langfuse \
  | jq '{export_active, base_url, api, api_resolved, project_id}'
```

| Field | Healthy 3.1 wiring |
|-------|--------------------|
| `export_active` | `true` |
| `base_url` | **your** 3.1 origin, not `https://cloud.langfuse.com` |
| `api` | `auto` (requested) |
| `api_resolved` | **`ingestion`** |

Same facts on `/health` → `operational.observability.langfuse_api` / `langfuse_api_resolved`. Settings → **Langfuse Observability** shows `Transport: ingestion (auto)`.

Then run a query (or ingest). In the 3.1 UI, traces appear as **GENERATION** (LLM) and **SPAN** (everything else). `retriever` / `embedding` / `chain` are **not** first-class types on 3.1.1 — they land as SPAN. Rich types need ≥ 3.22 OTLP.

If the Cost column is `$0.00`, that is Langfuse’s model catalogue (LAW-124-12: EdgeQuake never emits cost). Optional: `make langfuse-sync-prices`.

---

## 5. Troubleshooting

| Symptom | Cause | Fix |
|---------|--------|-----|
| `api_resolved` is `otlp` on a 3.1 host | Probe did not see 404 (proxy/ingress swallowed the path) | Fix routing so `/api/public/otel/v1/traces` reaches Langfuse (404 is correct). Or set `EDGEQUAKE_LANGFUSE_API=ingestion`. |
| Traces on Cloud, not on-prem | `LANGFUSE_BASE_URL` empty/unset | Set the in-cluster / internal URL; empty → Cloud. |
| HTTP 401 on ingestion | Keys from a different Langfuse | Recreate keys **on this** project. |
| `export_active: true` but empty UI | Worker raced Prisma (first boot) | Restart `langfuse-worker` after web is Ready. Repo compose does this in `make langfuse-3.1-up`. |
| Forced `otlp` against 3.1 | `EDGEQUAKE_LANGFUSE_API=otlp` | Unset or `auto`. |
| Illegal `{retriever,embedding,chain}-create` | Old exporter | Current code maps those to `span-create` only. |

---

## 6. Upgrade path

When Langfuse is **≥ 3.22** (or Cloud / in-repo v4): leave `EDGEQUAKE_LANGFUSE_API=auto`. The probe no longer 404s; `api_resolved` becomes `otlp`. No EdgeQuake code change required.
