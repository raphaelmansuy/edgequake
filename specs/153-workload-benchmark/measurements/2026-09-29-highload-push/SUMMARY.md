# 2026-09-29-highload-push — pushed workload run

Target `https://demo.edgequake.com`, tenant `00000000-0000-0000-0000-000000000002`,
workspace `00000000-0000-0000-0000-000000000003`, EdgeQuake 0.28.3, LLM `mistral-small-latest`.
Server: GCE `elitizon-db` (`saas-app-001`, `us-central1-a`), **e2-medium** shared-core, 2 vCPU / 4 GB — see `gcp-server.json`.

| Card | Result |
|------|--------|
| H1 fresh, closed loop (burst) | knee **C=12** (95.8% on time, p50 17.1 s); first fail C=14 (85.7%) |
| H2 fresh, open loop (90 s/level) | all λ ≤ 0.6/s pass (`not_reached_in_range`) |
| H3 FAQ, cache | 100% cache hits up to C=32 |
| H4 soak at C=12, 240 s | **24.2% on time**, look-up step 18.5 s mean — burst level not holdable |
| H5 sustained ladder (150 s rest, 210 s load, first 120 s discarded) | sustainable: **C=8** (95%), λ=0.5/s (100%), C=5, λ=0.35, λ=0.25; not sustainable: **C=10** (42.9%), **λ=0.6/s** (63%), **C=12** (9.1%) |

Bottleneck: VM CPU / Postgres look-up (grew ~1.2 s → 20 s across H1) — not the external LLM (flat).
Sustained capacity ≈ **8 concurrent askers or ~30 fresh questions/min**; burst ≈ 12 askers for ~2 min.

Caveats: single repetition per level; earlier same-day run (n=12/level) gave knee C=8 / λ=0.25, so treat ±2 as run-to-run noise;
`docker stats` CPU% is per-vCPU and can exceed 200%; H5 was executed in two invocations (default plan, then C=10 / λ=0.6 / C=12) appended into `provider/sustain.json`.

Report: `EdgeQuake-Higher-User-Workload-Report.pdf` (vector charts; 300 DPI PNG + SVG in `charts/`).
