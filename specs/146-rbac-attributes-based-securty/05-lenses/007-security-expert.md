# Lens — Security Expert (SPEC-146)

## Outcome

Fail-closed document ABAC with hybrid AllowSetProvider (SQL + Cedar-for-classified), existence-hiding, provenance-gated GraphRAG, and audited break-glass via **`edgequake-audit`** — NIST SP 800-162 ACM (PEP/PDP/PIP/PAP).

## Control objectives

| ID | Objective |
|----|-----------|
| C1 | Unauthorized content never in LLM context |
| C2 | Unauthorized titles never in list/cite/autocomplete/SSE (**KV PEP**) |
| C3 | No Secret→Public graph pivot via shared hubs |
| C4 | No cross-principal cache / `ret_*` reuse |
| C5 | Workers cannot query; master key is break-glass |
| C6 | RLS backstop on SQL; app role cannot BYPASSRLS |
| C7 | No dual Cedar↔SQL evaluator fork |
| C8 | ABAC=1 requires auth on |

## PDP choice (locked)

**Hybrid AllowSetProvider (LAW-146-18):**

| Share mode | Evaluator |
|------------|-----------|
| `workspace` / `acl` / `owner_only` | SQL / set algebra |
| `classified` | Cedar in-process (`cedar-policy` 4.x) |

OPA sidecar rejected: network hop, per-hop cost, untyped Rego vs GraphRAG latency budget.

Cedar pros for classified: native Rust, schema-typed, deny/forbid, µs eval, compile once.

## Policy hygiene

```ascii
  Publish path (classified templates)
    draft Cedar → schema validate → dry-run matrix → immutable version
    bump policy_version → invalidate authz caches
```

Default deny. Explicit permits. Forbid for clearance / export_control mismatches.

## Break-glass

```ascii
  Master key (PrincipalId::Master) OR explicit break_glass binding
       │
       ├─ capability check passes
       ├─ AllowSet = all non-quarantined in workspace (or tenant policy)
       ├─ edgequake-audit row REQUIRED (who, why, when, doc ids hash)
       ├─ UI banner role=alert
       └─ 403 path for non-BG when capability missing
          (not 404 — capability denial)
```

## Prompt injection

Authz is never delegated to the LLM. Corpus text cannot grant `document:read`. System/tool messages must not include denied titles or counts (`status_counts` authorized-only).

## Residual

Timing equalization best-effort. Misconfigured templates — mitigate with dry-run + defaults. KV list miss remains the highest operational risk (R1).

## Cross-refs

- Threat model → [../11-threat-model.md](../11-threat-model.md)  
- Laws → [../00-first-principles.md](../00-first-principles.md)  
- Catalog → [../12-role-attribute-catalog.md](../12-role-attribute-catalog.md)  
- Honest → [../13-honest-assessment.md](../13-honest-assessment.md)  
- Roadblocks → [../14-roadblocks.md](../14-roadblocks.md)  
