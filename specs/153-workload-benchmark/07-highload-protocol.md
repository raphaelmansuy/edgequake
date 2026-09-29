# 07 — High-load user simulation protocol

Parent: [README](README.md) · Base protocol: [04](04-protocol.md) · Laws: [01](01-first-principles.md) · Shapes: [05](05-workloads.md)

Executable extension of SPEC-153 for **concurrent users asking questions** on the
shared demo. Pass/fail = report card complete, **machines documented**, and
internally consistent (LAW-153-8). Knee may be `not_reached_in_range`.

## 1. Purpose

Answer in business language:

> About how many people can ask **new** questions at once on the demo tenant
> and still finish within ~25 seconds?

Layers under test: **L0** (admission) · **L7** (retrieve) · **L8** (generate).  
Not Acc (SPEC-001). Not DB physics (SPEC-090). No Provider ingest on shared demo.

## 2. Inheritance

| Law | How this protocol uses it |
|-----|---------------------------|
| LAW-153-1 | Report `tokens_used` as aggregate API field; do not invent embed+llm sums |
| LAW-153-2 | Capacity = goodput at knee (≥90% attainment) |
| LAW-153-3 | Record wall sojourn; service from `stats.total_time_ms` when present |
| LAW-153-4 | Provider-only card (live demo); pair with System ingest from hybrid run if needed |
| LAW-153-5 | Shape `query-chat-v1`; unique prompts for cold |
| LAW-153-6 | H1/H2 cold unique; H3 warm FAQ is a **separate** card |
| LAW-153-7 | Sweep only concurrency or λ; pin tenant/workspace/models |
| LAW-153-8 | Percentiles; USE via errors/429; proof ladder |

## 3. Topology (client vs SUT)

```text
  [Load generator — client laptop / CI agent]
      tools/bench153  ·  HTTPS
           |
           |  POST /api/v1/query
           |  X-API-Key + X-Tenant-ID + X-Workspace-ID
           v
  [SUT — https://demo.edgequake.com]
      Axum API + task workers + Postgres (AGE + pgvector)
      + LLM / embedding providers
      tenant  00000000-0000-0000-0000-000000000002
      workspace 00000000-0000-0000-0000-000000000003
```

Never publish client CPU as “demo capacity.” Never omit which machine generated load.

## 4. Machine inventory (required every run)

Write `measurements/<run_id>/machines.json` with two objects.

### 4.1 Load generator (`role: load_generator`)

| Field | How collected |
|-------|----------------|
| hostname | `hostname` |
| os / arch | `uname -a` / `uname -m`; macOS also `sw_vers` |
| cpu_model, cpu_logical | `sysctl -n machdep.cpu.brand_string hw.logicalcpu` (macOS) |
| mem_gb | `hw.memsize` / 2^30 |
| python | `python3 --version` |
| harness | package `bench153` + `git rev-parse --short HEAD` when available |
| network_egress | public IP (best-effort) + note `client_to_sut: internet_https` |
| timezone, started_at_utc | local TZ name + ISO-8601 UTC |

### 4.2 System under test (`role: sut`)

Record the **exact GCP host**, not "a cloud VM". Collected automatically by
`bench153.gcp_inventory` into `gcp-server.json` and embedded in `machines.json`.

| Field | How collected |
|-------|----------------|
| base_url, DNS check | `https://demo.edgequake.com`; resolved IP must equal the VM public IP |
| product_version, build | `GET /health` |
| storage_mode, postgres_major, pgvector, age | `/health.schema` |
| fleet_llm, fleet_embed | `/health.providers` |
| workspace_answer_llm | providers/models observed in unit stats |
| **GCP project, zone, instance, instance id** | `gcloud compute instances describe` |
| **machine type, vCPUs, shared-core flag, RAM** | `gcloud compute machine-types describe` |
| **CPU model / platform, OS, kernel** | IAP ssh (read-only): `/proc/cpuinfo`, `os-release`, `uname -r` |
| **disks (size, type)**, VPC/subnet, internal + public IP | describe + `gcloud compute disks list` |
| **containers + image tags** | IAP ssh (read-only): `sudo docker ps` |
| **Postgres settings** | `max_connections`, `shared_buffers`, `work_mem` via `psql` |
| auth | `X-API-Key` used; **never** store the secret |

Current demo host (SPEC-148): GCE VM **`elitizon-db`**, project `saas-app-001`,
zone `us-central1-a`, **e2-medium** (2 shared vCPU, 4 GB), AMD EPYC 7B12, Debian 12,
Caddy + EdgeQuake API + Postgres(AGE+pgvector) + Web UI in Docker Compose, static IP
`34.135.165.171`. The Cloud Run service `edgequake-api` in the same project does **not** serve the demo.

## 5. Workload cards

Profiles: `quick` (smoke), `standard`, `push` (default for capacity work).

| ID | Shape | Load model | `push` parameters | Stop rule |
|----|-------|------------|-------------------|-----------|
| **H1** | `query-chat-v1` cold unique | Closed-loop concurrency | C ∈ {2,4,6,8,10,12,14,16,20,24,32}; n = max(16, 2C) ≤ 48 | knee = first failing level; **explore-beyond-knee** continues up to 3 failing levels |
| **H2** | same | Open-loop arrival rate | λ ∈ {0.10,0.15,0.20,0.25,0.30,0.35,0.40,0.50,0.60} asks/s; steady **90 s** each | knee = first failing λ; continues up to 2 failing levels |
| **H3** | warm FAQ | Closed-loop | 3 fixed prompts; C ∈ {16,32}; n=32 | separate card; report `answer_cache_hit_rate` |
| **H4** | cold unique | Closed-loop, timed | C = H1 knee; **240 s** non-stop | stability: last-third median ≤ 1.15× first-third and attainment ≥ 0.90 |

| **H5** | cold unique | Sustained ladder (`--profile sustain`), one level at a time | `C=8`, `λ=0.5/s`, `C=5`, `λ=0.35/s`, `λ=0.25/s`; each level = **150 s idle rest** then **210 s** load | level is *sustainable* if the **steady window** (first 120 s discarded) has attainment ≥ 0.90, errors ≤ 5%, < 3 HTTP 429 |

**Burst vs sustained (why H5 exists).** GCE shared-core machines (`e2-micro/small/medium`) are guaranteed only a
fraction of CPU time in total (e2-medium: 100% of one vCPU across its 2 vCPUs) and may burst to 100% per vCPU for
≈ 120 s at 100% utilisation using accumulated credit (Google Cloud docs, “E2 shared-core machine types”). A sweep
of 30–90 s per level therefore measures the **burst** ceiling. The 240 s soak (H4) at the burst knee exposed this
(latency doubled after ~2 min). H5 measures the **sustained** ceiling: rest the VM so credit refills, load for longer than the
burst window, and judge only the steady window. Both numbers are reported; only the sustained one may be used for
"how many people can it serve all day".

**Hard safety brakes (always on, even when exploring past the knee):** error rate > 25%,
≥ 10 HTTP 429, or attainment < 10% → stop the sweep.

**SLO hypothesis (conjunction):**

- `oracle_ok`: HTTP 200 + non-empty answer  
- `slo_total_ok`: `stats.total_time_ms` (else wall) ≤ **25000** ms  
- Unit is **good** only if both hold; cold unique + cache hit ⇒ not good  

**Attainment** = good / completed in the steady window.  
**Knee** = last C (or λ) with attainment ≥ 0.90. If all levels pass: `not_reached_in_range`.

Little’s law reminder: past the knee, raising C inflates sojourn and can **lower** goodput.

## 6. Oracle and cold purity

- Cold (H1/H2): every prompt includes a unique token (`Unique=…`) so answer cache should miss.
- Record `answer_cache_hit` and `generation_time_ms` from API stats.
- Warm (H3): seed fixed prompts, then fire repeats — expect high cache hit rate.

## 7. Preflight

```bash
curl -sf https://demo.edgequake.com/live
curl -sf https://demo.edgequake.com/ready
curl -sf https://demo.edgequake.com/health | tee measurements/<run_id>/preflight-health.json
# ready==true, migration_required==false
# Collect machines.json (load_generator + sut)
```

Auth: one successful authenticated query. **401/403 → BLOCKED** (abort).

## 8. Commands

```bash
export BENCH153_RUN_DIR=specs/153-workload-benchmark/measurements/YYYY-MM-DD-highload
export BENCH153_API_KEY_FILE=/path/to/key   # chmod 600; scrub after
make bench153-provider-highload        # BENCH153_PROFILE=push (default): H1–H4
BENCH153_PROFILE=sustain make bench153-provider-highload   # H5, appended to the same run dir (~30 min)
make bench153-highload-report
```

While the cards run, `bench153.vm_telemetry` streams **read-only** VM samples over IAP ssh
(`/proc/stat`, `/proc/loadavg`, `docker stats`) every ~4 s into `telemetry.jsonl`. Nothing is written
on the VM. This answers *is the VM or the LLM the bottleneck?* (USE method, LAW-153-8). Use `--no-gcp` to skip it.

Artifacts: `machines.json`, `gcp-server.json`, `telemetry.jsonl`, `env.json`,
`provider/units.jsonl`, `provider/summary.json`, `provider/sustain.json`, `provider/units-sustain.jsonl`, `telemetry-sustain.jsonl`, `summary.json`,
`charts/*.svg` (vector, embedded in the PDF) + `charts/*.png` (**300 DPI**),
`EdgeQuake-Higher-User-Workload-Report.pdf` (plain-English protocol, architecture / pipeline / load-model
diagrams, and data charts).

## 9. Pass / fail

| Result | Meaning |
|--------|---------|
| **PASS** | H1–H3 attempted; machines.json complete; tenant/workspace echoed; knee or stop-guard recorded |
| **FAIL** | Missing machine inventory, mixed cold/warm in one attainment, Acc used as oracle |
| **BLOCKED** | Demo not ready / auth failure |

## 10. Safety (shared demo)

- Cap **C ≤ 32**; open-loop steady **90 s** per λ (push profile), λ ≤ 0.6/s; H5 ≤ 210 s per level with 150 s idle between levels.
- Container `docker stats` CPU% is relative to one vCPU and can exceed 200% on this 2-vCPU VM; use whole-VM CPU% (`/proc/stat`) to judge saturation.
- Unique cold prompts only for H1/H2.
- No document upload, delete, or Vision re-OCR.
- Scrub API key files; never put secrets in `machines.json` or the PDF.
