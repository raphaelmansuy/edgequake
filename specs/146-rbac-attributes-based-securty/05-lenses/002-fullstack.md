# Lens — Full Stack Developer (SPEC-146)

## Outcome

One authz path from WebUI → API PEP → query/storage PEP, with DRY components and no handler-local ACL copies.

## Stack map

```ascii
  WebUI
    SecurityFields (upload) · MembersCard · RolesCard · AttrsCard · PolicyCard
    existence-hiding empty states · quarantine chip
         │
         v
  edgequake-api PEPs
    documents · query · graph · mcp · members · policies · parse jobs
         │
         v
  edgequake-authz (AuthzDecision, AllowSetProvider)
         │
         +──► edgequake-query (modes, hops, caches)
         +──► edgequake-storage (typed ANN JOIN, RLS GUC)
         +──► edgequake-pipeline (label mint, worker principal)
```

## Implementation rules

1. **DIP** — handlers take `Arc<dyn AuthzDecision>`; never import Cedar types in handlers.  
2. **DRY** — `existence_hiding_404()`, `intersect_document_filter()`, `cache_key_authz()`; reuse `edgequake-audit`.  
3. **Feature flag** — `EDGEQUAKE_DOC_ABAC`; when off, skip allow-set; when on, auth required (LAW-146-20).  
4. **OpenAPI first** for members/policies; regenerate `schema.d.ts`.  
5. **Unify roles** in WebUI + API DTOs in M0 before UI polish.  
6. **KV list PEP** — filter `document_metadata_scan` results; do not rely on RLS for list (LAW-146-17).  
7. **Dual-write labels** — SQL columns + KV metadata at admit (F-146-36).  
8. **M1 split** — M1a schema/PDP/list; M1b members/upload UI (R7).  
9. **LAW-146-18** — SQL for workspace/acl/owner_only; Cedar only for classified.

## Touch list (primary)

| Area | Files (indicative) |
|------|--------------------|
| New crate | `edgequake/crates/edgequake-authz/` |
| Permissions | `edgequake-auth/src/rbac.rs` |
| Audit | `edgequake-audit` (extend, don’t fork) |
| Middleware | `edgequake-api/src/middleware.rs` |
| Documents | `handlers/documents/**`, `document_metadata_scan` |
| Query | `handlers/query/**`, `edgequake-query/**` |
| MCP | `mcp/gateway/**` |
| Upload UI | `document-dropzone.tsx`, `pdf-upload-form-data.ts` |
| Settings | cards on **both** `(dashboard)/settings` and `w/[slug]/settings` |

## Anti-patterns

```ascii
  ✗ if role == admin { return all_docs }
  ✗ trust document_filter.ids without ∩ allow-set
  ✗ post-filter only on Mix
  ✗ cache key without principal when ABAC on
  ✗ stream citations then redact
  ✗ assume RLS hides KV list titles
  ✗ dual Cedar↔SQL “equivalent” evaluators
  ✗ UUID-only ACL for master-api-key
```

## Test ownership

| Layer | Owner |
|-------|-------|
| Cedar / AllowSet unit | authz crate |
| PEP e2e (KV list + SQL) | edgequake-api postgres tests |
| ANN/hop | storage + query |
| Playwright | webui `e2e/spec146-*` |

## Cross-refs

- Architecture → [../04-target-architecture.md](../04-target-architecture.md)  
- Plan → [../07-implementation-plan.md](../07-implementation-plan.md)  
- Code as-is → [../03-code-as-is.md](../03-code-as-is.md)  
- Roadblocks → [../14-roadblocks.md](../14-roadblocks.md)  
- Honest → [../13-honest-assessment.md](../13-honest-assessment.md)  
