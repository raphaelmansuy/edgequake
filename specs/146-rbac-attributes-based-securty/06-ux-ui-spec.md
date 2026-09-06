# 06 — UX / UI Spec (SPEC-146)

Normative for WebUI when SPEC-146 is funded. Folds zz-raw UX lock (5 Sep 2026).

## Principles

1. Existence-hiding over “access denied” for resource IDs.  
2. Progressive disclosure: workspace share mode → Security collapsed.  
3. Same empty copy for zero-authz and true-empty.  
4. No PDP internals in user-facing errors.  
5. DRY: one `SecurityFields` across admit paths.

## Documents — navigation

```ascii
  ┌──────────────────────────────────────────────────────────────────┐
  │ Documents                                                         │
  ├──────────────────────────────────────────────────────────────────┤
  │ [Search titles]  [Status ▾]  [Classification ▾]  [Refresh]        │
  ├──────────────────────────────────────────────────────────────────┤
  │ ┌ Dropzone ────────────────────────────────────────────────────┐ │
  │ │  Drop PDF / MD / TXT                                           │ │
  │ │  Parser [Auto▾]  Vision […]                                   │ │
  │ │  ┌ Security ───────────────────────────────────────────────┐ │ │
  │ │  │ Classification [Internal ▾]  Share [Workspace ▾]         │ │ │
  │ │  │ Project [———— ▾]  ☐ PII  ☐ Export control                │ │ │
  │ │  │ ACL (if share=acl): [+ Add principal]                    │ │ │
  │ │  └──────────────────────────────────────────────────────────┘ │ │
  │ └──────────────────────────────────────────────────────────────┘ │
  │ ┌ Table ───────────────────────────────────────────────────────┐ │
  │ │ Title          Status       Class        Share      Owner    │ │
  │ │ handbook.pdf   Completed    Internal     Workspace  alice    │ │
  │ │ payroll.pdf    Quarantined  Secret       Classified —        │ │
  │ │   ↳ chip + Retry labels                                       │ │
  │ │ (no rows for unauthorized docs)                               │ │
  │ └──────────────────────────────────────────────────────────────┘ │
  └──────────────────────────────────────────────────────────────────┘
```

## Document detail — security panel

```ascii
  ┌ Side-by-side viewer ──────────────────────────────┐
  │ PDF │ Markdown                                      │
  ├─────────────────────────────────────────────────────┤
  │ Security                                            │
  │  Classification / Share / Project / Flags           │
  │  ACL list (acl mode)                                │
  │  [Edit labels] if document:set_labels               │
  │  QuarantineBanner if quarantined                    │
  └─────────────────────────────────────────────────────┘
```

## Query

```ascii
  Query page
    Document picker popover
      └─ GET search returns authorized titles only
    Answer citations
      └─ omit unauthorized; never grey Restricted
    Empty
      └─ "No matching results." (+ refine help only)
```

## Graph explorer

```ascii
  Neighborhood view
    nodes/edges from authorized provenance only
    degree badge = authorized degree (or hide)
    popular labels = authorized set or empty
```

## Settings — security cluster

```ascii
  /settings  (and /w/[slug]/settings)
  ├── User management     (fix role labels → admin/user/readonly)
  ├── Workspace members   NEW
  │     table: user, membership role, workspace roles, attrs summary
  ├── Roles               NEW
  │     builtin locked; custom → permission checklist
  ├── Attributes          NEW
  │     catalog CRUD; principal attr editor; OIDC claim map (advanced)
  ├── Policies            NEW
  │     template gallery → activate
  │     Advanced: Cedar editor + Validate + Publish version
  └── Break-glass / Audit NEW
        enable BG session; audit table
```

## Wireframes — members

```ascii
  ┌ Members ─────────────────────────────────────────────┐
  │ [Invite / Add]                                        │
  │ Principal     Membership   WS roles      Clearance    │
  │ alice@…       admin        admin         secret       │
  │ bob@…         member       editor        internal     │
  │ svc-ingest    —            ingestion_service  —       │
  └───────────────────────────────────────────────────────┘
```

## Copy deck (normative)

| Situation | Copy |
|-----------|------|
| Empty query/browse | “No matching results.” |
| 404 document | “Document not found.” |
| 403 ingest | “You don’t have permission to upload documents.” |
| 403 policy | “You don’t have permission to manage policies.” |
| Quarantine | “Security labels could not be applied. This document isn’t searchable until labeling succeeds.” |
| Break-glass banner | “Break-glass session active. Actions are audited.” |

## States matrix

| Surface | Loading | Empty | Error | Quarantine | BG |
|---------|---------|-------|-------|------------|-----|
| Doc table | skeleton | empty state | toast | chip+retry | banner |
| Query | stream | empty state | toast | n/a | banner |
| Graph | spinner | empty graph | toast | n/a | banner |
| Settings cards | skeleton | CTA add | inline | n/a | banner |

## A11y

- All selects labeled; chip text not color-only.  
- Focus order: SecurityFields before Admit.  
- `aria-live` for quarantine and break-glass.

## Evidence (when implemented)

Capture before/after for upload SecurityFields, empty query, quarantine chip, settings members — multi-viewport 1440/768/375 (SPEC-101 style).

## Cross-refs

- UX lens → [05-lenses/004-ux.md](05-lenses/004-ux.md)  
- Front → [05-lenses/005-front-designer.md](05-lenses/005-front-designer.md)  
- Doc manager → [05-lenses/006-document-manager.md](05-lenses/006-document-manager.md)  
- E2E → [08-e2e-test-matrix.md](08-e2e-test-matrix.md)  
