# Iteration 66 - ORIENT: Next Priority Assessment

**Date:** 2026-01-09

## Current State

- Fixed 5 collisions (FEAT0701-0705)
- 42 duplicates remaining
- 115 undocumented features

## Remaining Work Categories

### Category A: Intentional Overloading (Leave As-Is)

These represent the same feature across different layers:

| FEAT ID  | Occurrences | Across                      | Decision                         |
| -------- | ----------- | --------------------------- | -------------------------------- |
| FEAT0001 | 5           | types/stores/components/API | Keep - Document as cross-cutting |
| FEAT0007 | 5           | types/stores/components/API | Keep - Document as cross-cutting |
| FEAT0101 | 3           | stores/components           | Keep                             |
| FEAT0202 | 4           | stores/components           | Keep                             |
| FEAT0205 | 2           | stores/components           | Keep                             |
| FEAT0301 | 3           | 3 query components          | Keep                             |

**Total: ~20 occurrences across ~6 unique IDs - not collisions, just cross-cutting**

### Category B: API Client Collisions (FIXED ✅)

- FEAT0701-0705 migrated to FEAT0770-0774

### Category C: Namespace Violations (Priority)

Frontend features using backend namespace:

| Old ID   | Feature          | Wrong NS | Should Be        |
| -------- | ---------------- | -------- | ---------------- |
| FEAT0501 | JWT tokens       | LLM      | Auth UI (0870)   |
| FEAT0504 | Tenant context   | LLM      | Providers (0861) |
| FEAT0505 | Session mgmt     | LLM      | Auth UI (0871)   |
| FEAT0506 | Tenant switching | LLM      | Providers (0862) |
| FEAT0801 | Per-doc cost     | Auth     | Cost (0850)      |
| FEAT0803 | Workspace cost   | Auth     | Cost (0851)      |
| FEAT0804 | Cost breakdown   | Auth     | Cost (0852)      |

**Total: 7 IDs to migrate**

### Category D: Other Collisions

Various other duplicates found - need investigation.

## Recommended Next Iteration (67)

**Focus: Fix Category C - Auth/Tenant namespace violations**

Migrate:

1. FEAT0501 → FEAT0870 (JWT tokens)
2. FEAT0504 → FEAT0861 (Tenant context)
3. FEAT0505 → FEAT0871 (Session management)
4. FEAT0506 → FEAT0862 (Tenant switching)

This will fix namespace violations and reduce duplicates.

## Updated Namespace Allocation

| Range         | Module                      | Team     |
| ------------- | --------------------------- | -------- |
| FEAT0770-0779 | API Client (Chat/Streaming) | Frontend |
| FEAT0850-0859 | Cost Management             | Frontend |
| FEAT0860-0869 | WebUI Providers             | Frontend |
| FEAT0870-0879 | Auth UI                     | Frontend |
