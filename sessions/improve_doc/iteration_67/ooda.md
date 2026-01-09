# Iteration 67 - Auth/Tenant Namespace Migration

**Date:** 2026-01-09
**Objective:** Fix FEAT0501-0506 namespace violations (LLM → Auth UI/Providers)

## OBSERVE

### Files Containing FEAT050X Before Migration

| File                  | Old ID   | Feature                    |
| --------------------- | -------- | -------------------------- |
| use-auth-store.ts     | FEAT0501 | JWT token management       |
| use-auth-store.ts     | FEAT0505 | Token expiration detection |
| use-tenant-store.ts   | FEAT0504 | Multi-tenancy              |
| use-tenant-store.ts   | FEAT0506 | Tenant context persistence |
| use-tenant-context.ts | FEAT0504 | Multi-tenancy hook         |
| tenant-provider.tsx   | FEAT0504 | Multi-tenancy provider     |
| header.tsx            | FEAT0504 | Tenant selector            |
| types/index.ts        | FEAT0501 | Auth types                 |
| edgequake.ts          | FEAT0501 | Auth API                   |

### Namespace Violation Analysis

All these features are **frontend-specific** but use **backend LLM namespace (05XX)**.

## ORIENT

### Migration Mapping

| Old ID   | New ID   | Feature                          | Namespace |
| -------- | -------- | -------------------------------- | --------- |
| FEAT0501 | FEAT0870 | JWT tokens, Auth types, Auth API | Auth UI   |
| FEAT0504 | FEAT0861 | Multi-tenancy                    | Providers |
| FEAT0505 | FEAT0871 | Token expiration                 | Auth UI   |
| FEAT0506 | FEAT0862 | Tenant persistence               | Providers |

## DECIDE

Execute namespace migration for all 9 occurrences across 7 files.

## ACT

### Files Modified

```
M edgequake_webui/src/stores/use-auth-store.ts
  FEAT0501 → FEAT0870
  FEAT0505 → FEAT0871

M edgequake_webui/src/stores/use-tenant-store.ts
  FEAT0504 → FEAT0861
  FEAT0506 → FEAT0862

M edgequake_webui/src/hooks/use-tenant-context.ts
  FEAT0504 → FEAT0861

M edgequake_webui/src/providers/tenant-provider.tsx
  FEAT0504 → FEAT0861

M edgequake_webui/src/components/layout/header.tsx
  FEAT0504 → FEAT0861

M edgequake_webui/src/types/index.ts
  FEAT0501 → FEAT0870

M edgequake_webui/src/lib/api/edgequake.ts
  FEAT0501 → FEAT0870
```

### Verification

```bash
grep -r "@implements FEAT050[1-6]" edgequake_webui/src/
# Result: No matches - all migrated

grep -r "@implements FEAT08[67][0-9]" edgequake_webui/src/ | wc -l
# Result: 16 matches - new IDs in place
```

## Metrics

| Metric              | Before | After                                           |
| ------------------- | ------ | ----------------------------------------------- |
| FEAT050X violations | 9      | 0                                               |
| FEAT086X/087X uses  | 8      | 16                                              |
| Duplicates          | 42     | 42 (no change - fixed namespace, not collision) |

## Handoff to Iteration 68

Next: Migrate FEAT0801-0804 (Cost features in Auth namespace → Cost namespace 085X)
