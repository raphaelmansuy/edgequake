---
title: "Upgrade to EdgeQuake v0.31.0"
description: "Per-release upgrade notes for EdgeQuake v0.31.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.31.0

> **From:** v0.30.0 · **To:** v0.31.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`, `edgequake-keycloak`)

Minor cut: **SPEC-160** decision extraction as a **preview**. Schema train
moves **165 → 166**. Run migrate before `/ready` is 200. From ≤0.29.0, migrate
to 165 first ([upgrade-to-0.30.0.md](upgrade-to-0.30.0.md)), then to 166 with
this cut.

The default extractor stays **llm**. Decision mode is selected per upload or as
a workspace default. `EDGEQUAKE_DECISION_ENABLED=0` blocks new decision
uploads. Existing decision graph rows stay. The two tables are additive.

**demo.edgequake.com:** this cut installs the 0.31.0 API and Web UI. Decision
mode stays a preview (uncalibrated gates). Password auth remains the demo
default; SSO stays opt-in from v0.30.0.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | **166** `decision_cache` and `decision_review` |
| Extraction | Mode word `decision` on upload and workspace; chat LLM is unchanged |
| Status | `GET /api/v1/decision/status` and `GET /api/v1/decision/models` (3s probe) |
| Acc | Same 2026-08-15 medical-mid attestation as 0.30.0; default `llm` path not re-scored; decision mode unscored |

## Sequence

```text
# Already on 0.30.0 (schema 165):
EDGEQUAKE_VERSION=0.31.0 docker compose pull
# migrate Job / edgequake migrate (applies 166), then API
EDGEQUAKE_VERSION=0.31.0 docker compose up -d
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.31.0", schema.latest_version 166, pending_count 0
curl -sf localhost:8080/api/v1/decision/status
# host and model fields; never hangs past 3s
```

## Residuals (not release blockers)

- `openai_logprobs` / `llama-server` is refused at boot.
- No review screen. Review rows are stored and counted.
- Gate presets stay **Uncalibrated**. The two-document probe is in
  [w8-report.md](../../specs/160-tev1/measurements/w8-report.md). Treat
  `tev1:0.8b` at `balanced` as a smoke option.
- SPEC-001 Acc was not re-run for this cut.

Guide: [Decision extraction](../concepts/decision-extraction.md).
