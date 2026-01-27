# Iteration 68 - Cost Namespace Migration

**Date:** 2026-01-09
**Objective:** Fix FEAT0801-0804 namespace violations (Auth → Cost 085X)

## OBSERVE

### Files Containing FEAT080X Before Migration

| File              | Old IDs          | Features                      |
| ----------------- | ---------------- | ----------------------------- |
| types/cost.ts     | 0801, 0804       | Per-doc cost, Token breakdown |
| use-cost-store.ts | 0801, 0802, 0803 | Per-doc, Real-time, Workspace |
| use-cost.ts       | 0801, 0803       | Per-doc, Workspace            |

### Collision Discovery

During migration to 085X, discovered:

- `app/page.tsx` already uses FEAT0850-0852 for Dashboard features
- These Dashboard features needed relocation

## ORIENT

### Migration Mapping

| Old ID   | New ID   | Feature                          |
| -------- | -------- | -------------------------------- |
| FEAT0801 | FEAT0850 | Per-document cost tracking       |
| FEAT0802 | FEAT0851 | Real-time ingestion cost updates |
| FEAT0803 | FEAT0852 | Workspace cost summary           |
| FEAT0804 | FEAT0853 | Token usage breakdown            |

### Secondary Migration (Dashboard collision)

| Old ID   | New ID   | Feature                       |
| -------- | -------- | ----------------------------- |
| FEAT0850 | FEAT0900 | Dashboard overview with stats |
| FEAT0851 | FEAT0901 | Recent activity feed          |
| FEAT0852 | FEAT0902 | Quick action shortcuts        |

## ACT

### Files Modified

```
M edgequake_webui/src/types/cost.ts
  FEAT0801 → FEAT0850
  FEAT0804 → FEAT0853

M edgequake_webui/src/stores/use-cost-store.ts
  FEAT0801 → FEAT0850
  FEAT0802 → FEAT0851
  FEAT0803 → FEAT0852

M edgequake_webui/src/hooks/use-cost.ts
  FEAT0801 → FEAT0850
  FEAT0803 → FEAT0852

M edgequake_webui/src/app/page.tsx
  FEAT0850 → FEAT0900
  FEAT0851 → FEAT0901
  FEAT0852 → FEAT0902
```

### Updated Namespace Allocation

| Range         | Module          | Team     |
| ------------- | --------------- | -------- |
| FEAT0850-0859 | Cost Management | Frontend |
| FEAT0860-0869 | WebUI Providers | Frontend |
| FEAT0870-0879 | Auth UI         | Frontend |
| FEAT0900-0909 | Dashboard       | Frontend |

## Metrics

| Metric                    | Before | After |
| ------------------------- | ------ | ----- |
| FEAT080X violations       | 7      | 0     |
| FEAT085X uses (Cost)      | 0      | 7     |
| FEAT090X uses (Dashboard) | 0      | 3     |
| Duplicates                | 43     | 42    |

## Verification

```bash
grep -r "@implements FEAT080[1-4]" edgequake_webui/src/
# Result: No matches

grep -r "@implements FEAT085[0-9]" edgequake_webui/src/ | wc -l
# Result: 7 (all Cost features)
```

## Handoff to Iteration 69

Next: Generate documentation stubs for 120 undocumented features using generate_registry.py
