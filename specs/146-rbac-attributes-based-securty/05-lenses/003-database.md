# Lens — Database Expert (SPEC-146)

## Outcome

Postgres remains SSOT for attrs, ACL, policies, and RLS **backstop**. Document **list** is KV — RLS does not hide titles (LAW-146-17 / R1). AGE stays provenance-rich without classifying hubs. Typed ANN pre-filters by document allow-set without killing HNSW.

## Schema priorities

1. Typed columns on `documents` for hot filters (`share_mode`, `classification`, `security_status`).  
2. `document_acl` with **tagged** `(principal_kind, principal_id)` — not UUID-only (LAW-146-19).  
3. Policy versions immutable; hash for integrity; Cedar text for **classified** only.  
4. Prefer **JOIN `chunks`** for ANN; denorm `document_id` only if measured (R4).  
5. FORCE RLS retained; app role no BYPASSRLS.  
6. Allow-set SQL for `workspace|acl|owner_only`; no parallel Cedar SQL compiler (LAW-146-18).

## Allow-set SQL patterns

```ascii
  Hot path:
    WITH allow AS (SELECT unnest($allow::uuid[]) AS id)
    SELECT ... FROM chunk_embeddings ce
    JOIN chunks c ON c.id = ce.chunk_id
    JOIN allow a ON a.id = c.document_id
    WHERE ce.workspace_id = $ws AND ce.model_id = $m
    ORDER BY ce.embedding <=> $q LIMIT k;

  Empty allow-set:
    return zero rows (fail-closed) — never omit JOIN

  List path (NOT SQL):
    document_metadata_scan → filter entries by allow-set in app
```

## Index plan

| Index | Why |
|-------|-----|
| `(workspace_id, share_mode, security_status)` | AllowSet builder |
| `(workspace_id, classification)` | Classified scans |
| `document_acl(principal_kind, principal_id)` | Reverse lookup |
| Existing chunk FK + HNSW on embeddings | ANN |
| GIN `source_ids` (AGE/relational) | Hop provenance |

## RLS backstop

```ascii
  TX start (PEP):
    SET LOCAL app.current_tenant_id
    SET LOCAL app.current_workspace_id
    SET LOCAL app.current_user_id
    SET LOCAL app.allow_document_ids = '{uuid,...}'  -- or ''

  Policy: document visible iff workspace match
          AND (ABAC off OR id = ANY(allow_document_ids))

  Reminder: KV list never sees this GUC — app PEP required
```

## Migration honesty

```ascii
  150_spec146_document_security_columns.sql
  151_spec146_document_acl.sql          -- tagged principals
  152_spec146_attribute_catalog.sql
  153_spec146_workspace_roles.sql
  154_spec146_policies.sql              -- Cedar for classified
  155_spec146_rls_document_allowlist.sql
  (+ optional 156 denorm chunk_embeddings.document_id)
```

Backfill `share_mode='workspace'`. Seed builtin roles + default Cedar templates per workspace.

## AGE rules

- Never put Secret classification solely on hub.  
- Occurrences/edges carry `source_ids`.  
- No per-principal graph tables in v1.

## Perf risks

| Risk | Mitigation |
|------|------------|
| Large `= ANY(allow)` | Cap / bitmap / temp table; measure (UNCONFIRMED) |
| HNSW + filter underfill | JOIN + iterative_scan; denorm only if measured (R4) |
| Policy compile | Once per version; Cedar only on classified subset (R3) |

## Cross-refs

- Data model → [../05-data-model.md](../05-data-model.md)  
- Embedding lens → [010-embedding-graph.md](010-embedding-graph.md)  
- Roadblocks → [../14-roadblocks.md](../14-roadblocks.md)  
- Honest → [../13-honest-assessment.md](../13-honest-assessment.md)  
- SPEC-091 / 098 inheritance  
