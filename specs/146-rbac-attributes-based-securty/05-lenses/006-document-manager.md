# Lens — Document Manager (SPEC-146)

## Outcome

Document managers can classify and share at upload, fix quarantine without re-uploading bytes, and trust list/detail/query surfaces never show out-of-policy titles.

## Document lifecycle + security

```ascii
  Admit
    │  SecurityFields mint
    ├─ OK ──────────────► pending → … → completed
    │                          │
    │                          └─ searchable iff security_status=ok
    │
    └─ classified miss ─► quarantined / labeling_failed
                           │
                           ├─ Retry labels → ok
                           └─ Delete

  Reprocess / reanalyze
    │  MUST preserve labels (EC-146-21)
    │  Widening labels requires document:set_labels
    v
  Same document_id + policy_etag bump if labels change
```

## Navigation map

```ascii
  /documents
      │
      ├─ Upload (labels)
      ├─ Table (authorized only)
      ├─ Detail / side-by-side
      │     ├─ Security panel (edit if permitted)
      │     └─ Quarantine banner
      ├─ Batch delete (authorized ids only)
      └─ Reprocess (labels preserved)

  /query
      └─ Document picker = authorized set
         Citations = authorized titles only

  /graph
      └─ Neighborhood = authorized provenance
```

## Batch upload

```ascii
  Batch of N files
    │
    ├─ Shared SecurityFields default applied to all
    ├─ Per-file override in review step (optional M1+)
    └─ Mixed classifications OK (EC-146-22)
       each document own ACL / share_mode
```

## Duplicate content_hash

Workspace unique hash for indexed docs (existing). Different classifications of same bytes:

- Prefer replace flow with explicit label confirmation, or  
- Reject second admit with clear error (no silent merge of Secret into Public row).

See EC-146-23.

## Manager checklist

| Task | Surface |
|------|---------|
| Set default workspace share mode | Workspace settings / policies |
| Label at upload | Dropzone SecurityFields |
| Fix quarantine | Detail → Retry labels |
| Share with ACL principals | Security panel / upload ACL |
| Verify viewer cannot see Secret | Impersonation test / second login e2e |

## Cross-refs

- UX UI → [../06-ux-ui-spec.md](../06-ux-ui-spec.md)  
- Edge cases → [../09-edge-cases.md](../09-edge-cases.md)  
- Data model → [../05-data-model.md](../05-data-model.md)  
