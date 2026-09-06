# Lens — Front Designer (SPEC-146)

## Outcome

Security UI feels native to EdgeQuake (shadcn, existing Document Manager / Settings patterns), progressive disclosure, no “compliance wallpaper.”

## Visual system

```ascii
  Classification chips (examples)
  ┌──────────┐ ┌───────────┐ ┌──────────────┐ ┌────────┐
  │ Public   │ │ Internal  │ │ Confidential │ │ Secret │
  │ outline  │ │ secondary │ │ default      │ │ destr. │
  └──────────┘ └───────────┘ └──────────────┘ └────────┘

  Share mode: subtle badge next to status
  Quarantine: warning variant + Retry
```

## Layout — Documents

```ascii
  ┌─ Toolbar ─────────────────────────────────────────────┐
  │ Search · Status · Classification filter (authorized)  │
  └───────────────────────────────────────────────────────┘
  ┌─ Dropzone ────────────────────────────────────────────┐
  │  drag files                                            │
  │  [PDF opts]  [Security ▾]                              │
  └───────────────────────────────────────────────────────┘
  ┌─ Table ───────────────────────────────────────────────┐
  │ Title        Status      Class      Share    Owner     │
  │ report.pdf   Completed   Internal   Workspace alice    │
  │ secret.pdf   Quarantined Secret     Classified  —      │
  │ (unauthorized rows simply absent)                      │
  └───────────────────────────────────────────────────────┘
```

## Layout — Settings security cluster

```ascii
  ┌─ Members ────────────────┐  ┌─ Roles ─────────────────┐
  │ table + invite           │  │ builtin + custom         │
  └──────────────────────────┘  └──────────────────────────┘
  ┌─ Attributes ─────────────┐  ┌─ Policies ──────────────┐
  │ catalog + principal edit │  │ templates | advanced     │
  └──────────────────────────┘  └──────────────────────────┘
```

## Components (DRY)

| Component | Reuse |
|-----------|-------|
| `SecurityFields` | Dropzone, batch, PDF dialog, edit labels |
| `ClassificationChip` | Table, detail, query picker |
| `QuarantineBanner` | Detail + table |
| `ExistenceEmptyState` | Documents, query, graph |
| `BreakGlassBanner` | App shell when BG session |

## Motion / density

- No celebratory animation on deny.  
- Dense tables OK; security filters use same filter bar pattern as status.  
- Advanced Cedar editor: monospace, validate button, diff of version.

## Responsive

- SecurityFields stack on mobile; ACL principal picker full-width sheet.  
- Settings cards single column &lt; 768px.

## Cross-refs

- UX → [004-ux.md](004-ux.md)  
- Document manager → [006-document-manager.md](006-document-manager.md)  
- SPEC-099 progressive disclosure  
