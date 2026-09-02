# SPEC-124 — Langfuse Support

> **Mission:** Give EdgeQuake first-class LLM observability via [Langfuse](https://langfuse.com/) — OTLP/HTTP export from `edgequake-observability`, env-only secrets, Settings deep-link, nested RAG traces — without replacing Prometheus or Jaeger.

## Short verdict

| Layer | Finding |
|-------|---------|
| Gap | Prometheus + optional OTLP **gRPC** exist; Langfuse requires **OTLP/HTTP** (≥ 3.22). Self-hosted **3.1.x** 404s that path → native ingestion fallback |
| Product | Operators cannot debug RAG quality/cost in an LLM-native UI; LightRAG already ships Langfuse |
| Fix posture | Dual export (gRPC Jaeger + HTTP Langfuse); wire unused `with_rag_generation_span`; Settings card + Open link |

```ascii
  Query / Ingest request
       │
       ├─ Prometheus /metrics          (unchanged)
       ├─ OTLP gRPC → Jaeger           (optional, existing)
       └─ Langfuse (LANGFUSE_* gated)
            ├─ OTLP HTTP ≥ 3.22 / Cloud
            └─ native ingestion on OTLP 404 (3.1.x fallback)
```

## Document map

```ascii
 00-why
   → 01-first-principles (LAW-124-*)
   → 02-cross-ref-matrix
   → 03-code-as-is
   → 04-target-architecture
   → 05-lenses/ (PO, fullstack, DB, UX, front, marketing, AI, observability)
   → 06-ux-ui-spec
   → 07-implementation-plan
   → 08-test-protocol
   → 09-acceptance
   → 10-edge-cases
   → 11-honest-assessment
   → 12-sessions-and-genai
   → 13-metadata-tokens-and-coverage
   → 14-observation-io-and-full-observe
   → 15-pipeline-observe-and-slugs
```

## Status board

| ID | Item | Status |
|----|------|--------|
| D0 | Langfuse skill vendored (`.github/skills/langfuse`) | Done |
| D1 | Doc pack | Done |
| I1 | OTLP/HTTP Langfuse exporter (dual with gRPC) | Done |
| I2 | Wire generation + pipeline spans | Done |
| I3 | Health/settings DTO + Settings card + Open link | Done |
| I4 | Sessions: conversation_id → Langfuse / GenAI attrs | Done |
| I5 | Tokens + full query/ingest coverage (cost never) | Done |
| I6 | Observation Input/Output on GenAI + key workflow spans | Done |
| I7 | DRY/SOLID SSOT + gleaning + session Open link + `make spec124-proof` | Done |
| I8 | Slugs additive + query/ingest pipeline observe | Done |
| T1 | Unit + API + Playwright + edge matrix | Done |
| T2 | CI-unfakable InMemory + stream/gleaning/pipeline contracts (`make spec124-proof`) | Done |
| T3 | Optional local Langfuse v4 Docker (`make langfuse-up` / `spec124-langfuse-e2e`) | Done |
| T4 | Langfuse 3.1.1 ingestion fallback (`make spec124-langfuse-3.1-e2e`: version pin, probe, `LangfuseIngestionExporter::export`) | Done |

## Related

- [SPEC-018](../018-observability/) — observability SSOT
- [`docs/OBSERVABILITY.md`](../../docs/OBSERVABILITY.md) — operator docs
- [12-sessions-and-genai.md](12-sessions-and-genai.md) — Sessions binding
- [13-metadata-tokens-and-coverage.md](13-metadata-tokens-and-coverage.md) — tokens / observation types / denylist
- [14-observation-io-and-full-observe.md](14-observation-io-and-full-observe.md) — Input/Output allowlist + gaps
- [15-pipeline-observe-and-slugs.md](15-pipeline-observe-and-slugs.md) — slugs + query/ingest pipeline observe
- [SPEC-145](../145-fix-truncated-logs/) — **I/O completeness amendment** (removes global 512 preview for generation + query-root; LAW-124-18 superseded for that class)
- [SPEC-123](../123-env-config-priority/) — config priority law (models); Langfuse is **env-only** (LAW-124)
- Skill: [`.github/skills/langfuse/`](../../.github/skills/langfuse/)
- Upstream: [Langfuse OTel](https://langfuse.com/integrations/native/opentelemetry), [best practices](https://langfuse.com/docs/observability/best-practices)

## Non-goals (v1)

- Langfuse Prompt Management / Playground migration
- In-product LLM-as-judge eval pipelines
- Storing Langfuse secrets in PostgreSQL
- Replacing Prometheus or removing Jaeger gRPC
- Emitting every sqlx/HTTP client span (noise)
