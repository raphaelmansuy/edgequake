# SPEC-163 — Onboarding and Provider Configuration

> **Status:** Implemented (WP-0..WP-8) on HEAD through migration **169**.  
> **Proof:** `make spec163-proof` (hermetic fake LLM) + `cargo test -p edgequake-secrets -p edgequake-fake-llm -p edgequake-api --lib provider_test --lib ssrf --lib locality`.  
> **Operator guide:** [`docs/getting-started/index.md`](../../docs/getting-started/index.md) · [`docs/providers/index.md`](../../docs/providers/index.md).  
> **Epochs:** schema **169** (`169_spec163_provider_connections.sql`); SPEC-150 train docs updated.

## What operators need to know

| Goal | Do this |
|------|---------|
| First start | `curl …/quickstart.sh \| sh` or `quickstart.sh --yes --provider ollama` |
| Configure a local server | Settings → Connections, or `POST /api/v1/connections` |
| Test a provider | `POST /api/v1/providers/test` or the Test button |
| Diagnose | `edgequake doctor` / `edgequake doctor --json` |
| Apply schema | `edgequake migrate` (169 is expand-only) |
| Encrypt keys | Set `EDGEQUAKE_SECRETS_KEY` (32-byte base64 or hex) before writing keys |

Pragmatic docs: [docs/providers/](../../docs/providers/) · Day-2: [11-ops-runbook.md](11-ops-runbook.md).

## Proof (honest bounds)

| Claim | What we actually ran |
|-------|----------------------|
| Hermetic onboarding | Fake LLM server (OpenAI + Anthropic + Ollama shapes) + unit/contract tests |
| Live oMLX / MLX / LM Studio | Opt-in `make spec163-local-matrix` — not CI |
| SPEC-150 epoch matrix through 169 | Train docs + migration file; full `make spec150-matrix` is operator-run |
| Time-to-first-answer | Baseline recorded in [measurements/baseline.json](measurements/baseline.json); hermetic budget 120s |
| Real-LLM quality | Out of scope (SPEC-001) |

Artifacts: [reports/](reports/) · [measurements/](measurements/).

## One-screen architecture

```text
  quickstart / wizard          serve
  ───────────────────          ─────
    detect local servers       honest /health probes
    create Connection          POST /providers/test
    test list+chat+embed       doctor (schema, roles, posture)
    assign workspace roles     encrypted keys at rest
                               SSRF check on custom URLs
```

## Success criteria

| ID | Criterion | Evidence |
|----|-----------|----------|
| S1 | Clone to first validated answer ≤ 3 commands, 0 manual env edits | compose e2e + measurements |
| S2 | Local OpenAI-shape and Anthropic-shape configurable from UI/API/CLI | contract tests |
| S3 | Every workspace model role configurable | connections + llm_roles |
| S4 | Provider health is truthful | fault-injection fake server |
| S5 | Keys encrypted, masked, never logged | secrets crate tests |
| S6 | SSRF denial matrix green; quickstart binds loopback | ssrf tests + compose |
| S7 | Docs gates: env registry generated | `scripts/generate_env_reference.py` |
| S8 | Schema 169 expand-only; train docs mention 169 | `make schema-train-docs` |

## Reading order

1. **Operate now:** [docs/providers/](../../docs/providers/) · [11-ops-runbook.md](11-ops-runbook.md)
2. **Why / laws:** [00-why](00-why.md) · [01-first-principles](01-first-principles.md)
3. **Past / defects:** [02-incident-catalogue.md](02-incident-catalogue.md) · [04-current-architecture.md](04-current-architecture.md)
4. **Target / build:** [06-target-architecture.md](06-target-architecture.md) · [08-implementation-plan.md](08-implementation-plan.md) · [09-test-proof-protocol.md](09-test-proof-protocol.md)
5. **Honesty:** [12-risks-honest-assessment.md](12-risks-honest-assessment.md)

## Non-goals

- Publishing workspace crates to crates.io.
- External KMS (interface leaves a hook via `EDGEQUAKE_SECRETS_KEY` rotation).
- Image-generation providers.
- Real-LLM answer quality (SPEC-001).
- Upstream `edgequake-llm` 0.11.0 publish — `from_connection` is implemented in-tree using public constructors; Anthropic runtime still sends `x-api-key` (Bearer is used on the test-connection probe).
