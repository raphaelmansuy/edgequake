# SPEC-018 E2E Proof Index

**Status:** Implemented (2026-06-05)  
**Run all proofs:** `./e2e/run_observability_proof.sh`

| Directory | Proof doc | Verifies |
|-----------|-----------|----------|
| [001-methodology/e2e](./001-methodology/e2e/001-framework-proof.md) | Framework + env contract | `EDGEQUAKE_LOG_FORMAT`, evidence standard |
| [002-cross-crate/e2e](./002-cross-crate/e2e/001-correlation-mvp-proof.md) | Cross-crate MVP | Request ID, metrics, propagation |
| [003-edgequake-api](./003-edgequake-api/e2e/001-request-id-middleware-proof.md) | API middleware | `observability_proof` tests |
| [004-edgequake-core](./004-edgequake-core/e2e/001-span-context-proof.md) | Core | Tracing macros present |
| [005-edgequake-pipeline](./005-edgequake-pipeline/e2e/001-log-levels-proof.md) | Pipeline | Crate builds with tracing |
| [006-edgequake-query](./006-edgequake-query/e2e/001-query-audit-proof.md) | Query | Handler extensions + audit event |
| [007-edgequake-storage](./007-edgequake-storage/e2e/001-storage-tracing-proof.md) | Storage | `cargo test -p edgequake-storage --lib` |
| [008-edgequake-pdf](./008-edgequake-pdf/e2e/001-pdf-tracing-proof.md) | PDF | Crate builds |
| [009-edgequake-auth](./009-edgequake-auth/e2e/001-auth-proof.md) | Auth | Crate tests |
| [010-edgequake-audit](./010-edgequake-audit/e2e/001-audit-wiring-proof.md) | Audit | `query_audit_logs` compiles |
| [011-edgequake-tasks](./011-edgequake-tasks/e2e/001-tasks-proof.md) | Tasks | Worker tracing |
| [012-edgequake-rate-limiter](./012-edgequake-rate-limiter/e2e/001-rate-limiter-proof.md) | Rate limiter | Middleware warn pattern |
| [013-edgequake-webui](./013-edgequake-webui/e2e/001-client-request-id-proof.md) | WebUI | Vitest `observability-client.test.ts` |
| [014-edgequake-llm](./014-edgequake-llm/e2e/001-header-propagation-proof.md) | LLM | API merges `PropagationHeaders` |
