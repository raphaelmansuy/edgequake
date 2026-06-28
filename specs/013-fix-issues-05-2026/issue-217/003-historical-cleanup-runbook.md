# Issue #217 — Historical entity-type cleanup runbook

## Problem

New ingestions enforce workspace `entity_types` after LLM extraction (#217 fix). **Existing graph nodes** created before enforcement may still carry free-form or legacy types (`Event`, `Technology`, etc.).

## When to run

- After upgrading to SPEC-013+ entity policy enforcement.
- When graph UI or queries show types outside the workspace allow-list.
- Before customer demos where entity-type filters must be trustworthy.

## Preconditions

- PostgreSQL storage (`DATABASE_URL` set).
- Workspace `entity_types` updated to the desired allow-list (see issue #216 API).
- Maintenance window if using full knowledge-graph rebuild (can be CPU/LLM heavy).

## Procedure

### 1. Enumerate affected workspaces

```bash
# List workspaces (per tenant) and inspect entity_types config
curl -s -H "X-Tenant-ID: $TENANT_ID" \
  "$EDGEQUAKE_API/api/v1/tenants/$TENANT_ID/workspaces" | jq '.[] | {id, name, entity_types}'
```

For each workspace, sample graph entity types (UI: Knowledge Graph, or API graph endpoints). Flag any type **not** in `entity_types`.

### 2. Choose remediation strategy

| Strategy | When | Impact |
|----------|------|--------|
| **A. Re-ingest documents** | Few docs, types wrong in pipeline output | Safer; uses current prompts + enforcement |
| **B. Rebuild knowledge graph** | Many docs, graph inconsistent | Faster than full re-upload; uses workspace LLM config |
| **C. Full workspace reset** | Test/dev only | Deletes graph + vectors; re-upload all sources |

### 3. Re-ingest path (recommended for production)

1. Set workspace `entity_types` to target list (PATCH workspace).
2. For each completed document: trigger re-process / force reindex (API or UI) **or** re-upload PDF with `force_reindex=true`.
3. Wait for pipeline `Completed` on all documents.
4. Re-run sample queries; verify entity types in graph match allow-list.

### 4. Rebuild knowledge graph path

1. Confirm workspace LLM + embedding providers are healthy (`GET /health`).
2. Use Web UI: Workspace → **Rebuild knowledge graph** (or equivalent API).
3. Monitor backend logs for extraction errors.
4. Spot-check 10 random entities: `entity_type` ∈ workspace allow-list.

### 5. Verification report (copy into proof doc)

Record per workspace:

- `workspace_id`, `entity_types` configured
- Document count re-processed
- Sample of 20 entity types before / after
- Query smoke test (hybrid mode, non-empty answer + sources)
- Pass/fail: **zero** entities outside allow-list

## Rollback

- Restore workspace config from backup if entity_types were changed incorrectly.
- Graph rebuild is destructive to in-graph merge state; keep DB snapshot before large rebuilds.

## Automated audit (SPEC-013 iteration 4)

```bash
export EDGEQUAKE_API=http://localhost:8080
make spec013-entity-type-audit TENANT_ID=<uuid> WORKSPACE_ID=<uuid>
# Or: python3 scripts/spec013_entity_type_audit.py --tenant-id ... --workspace-id ...
```

Exit code `1` = violations found (types outside workspace `entity_types`). Remediation (re-ingest / rebuild) is still manual — see steps above.

## Related proof

- [002-proof-and-evidence.md](002-proof-and-evidence.md) — new-ingest enforcement
- [../009-brutal-assessment.md](../009-brutal-assessment.md) — historical debt callout
