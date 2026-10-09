---
title: "Upgrade to EdgeQuake v0.33.0"
description: "Per-release upgrade notes for EdgeQuake v0.33.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.33.0

> **From:** v0.32.2 · **To:** v0.33.0 · **Schema:** **168 → 169** (expand-only)

SPEC-163 onboarding and provider configuration: encrypted Connections,
honest health, `edgequake doctor`, and a documented provider guide.

**Product crate pin on this branch may still read 0.32.2 until the tagged
cut.** Schema **169** is already on HEAD; run `edgequake migrate` before
expecting stored connections.

## Highlights

| Area | What changed |
|------|----------------|
| Connections | `provider_connections` table; admin CRUD at `/api/v1/connections` |
| Secrets | `EDGEQUAKE_SECRETS_KEY` (AES-256-GCM); keys write-only / masked |
| Health | Live probes in `/health` and `/models/health`; `POST /providers/test` |
| Doctor | `edgequake doctor [--json]` |
| Quickstart | `--yes --provider --base-url`; bind `127.0.0.1`; generated JWT |
| Docs | `docs/providers/` + generated env reference |
| Schema | **169** expand-only |

## Sequence

```text
# From v0.32.2 (schema 168):
edgequake migrate
# expect applied 169_spec163_provider_connections.sql
export EDGEQUAKE_SECRETS_KEY="$(openssl rand -base64 32)"
```

## Verify

```bash
edgequake doctor --json
curl -sf localhost:8080/health | jq '{status, security_posture, components}'
```

## Residuals (not release blockers)

- Full `make spec150-matrix` through 169 is operator-run (same as other specs).
- Real oMLX / MLX / LM Studio: `make spec163-local-matrix` (not CI).
- Upstream `edgequake-llm` 0.11.0 is not published; `from_connection` uses
  in-tree public constructors. Anthropic runtime still sends `x-api-key`
  (Bearer is used on the test-connection probe).
