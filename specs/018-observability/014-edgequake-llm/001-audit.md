# edgequake-llm (external) — Observability Audit

**Package:** `edgequake-llm` (crates.io, workspace pin ~0.6.20)  
**Scope:** Outbound LLM/embedding HTTP — **not in this repo**

---

## Executive Summary

Distributed tracing **exits EdgeQuake** at the LLM provider HTTP call. edgequake-llm v0.6.16+ supports **`with_extra_headers()`** for W3C and B2B headers. EdgeQuake API wires this **only when clients pass `extra_headers` in JSON** — not from incoming HTTP automatically.

Spec: [HEADER_PROPAGATION](../../edgequake-llm-update/HEADER_PROPAGATION.md)

---

## Propagation Chain (today)

```
  WebUI                API handler              edgequake-llm           OpenAI/Ollama
    │                      │                         │                      │
    │  no traceparent      │  QueryRequest           │  with_extra_headers  │
    │─────────────────────▶│  .extra_headers         │  (if Some)           │
    │                      │────────────────────────▶│─────────────────────▶│
    │                      │                         │                      │
    ╳                      ╳ optional JSON only      ╳                      ╳
```

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| LLM-OBS-001 | P1 | Manual header pass-through | `query_execute.rs`, `safety_limits.rs` | API harvests inbound headers |
| LLM-OBS-002 | P2 | No client span in EdgeQuake | External crate | Wrap with OTEL `SpanKind::Client` in API |
| LLM-OBS-003 | ✅ | Reserved header protection | HEADER_PROPAGATION SC-HEADER-02 | Document for operators |
| LLM-OBS-004 | P2 | Token usage not in traces | Provider responses | Span attributes `gen_ai.usage.*` (OTEL semconv) |

---

## Reserved Headers (do not override)

From spec: `Authorization`, `Content-Type`, etc. — caller `traceparent` is forwarded.

---

## Target (OTEL semantic conventions)

```
span: llm.chat
  attributes:
    gen_ai.system: openai | ollama
    gen_ai.request.model: gpt-5-nano
    gen_ai.usage.input_tokens: N
    gen_ai.usage.output_tokens: M
    http.request.header.traceparent: (injected)
```

---

## EdgeQuake Integration Points

| API location | Function |
|--------------|----------|
| `safety_limits.rs` | `create_safe_llm_provider_with_headers` |
| `providers/resolver.rs` | `LlmResolutionRequest.extra_headers` |
| `handlers/query/query_stream.rs` | Same for streaming |

---

## Verify (integration)

```bash
# After Phase 2 — trace should continue to provider
curl -X POST localhost:8080/api/v1/query \
  -H 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
  -H 'Content-Type: application/json' \
  -d '{"query":"test"}'
```
