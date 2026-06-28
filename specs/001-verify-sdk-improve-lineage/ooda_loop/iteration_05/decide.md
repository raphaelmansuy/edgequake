# OODA Iteration 05 - DECIDE

**Decision ID**: OODA-05-DEC  
**Date**: 2026-02-15  
**Phase**: Phase 2 - Python SDK Excellence

---

## Linting Status Discovered

### Ruff (Style/Imports)

```text
Found 7 errors (all fixable with --fix)
```

- All errors are unused imports (F401)
- **Decision**: Auto-fix with `ruff check --fix`

### Mypy (Type Safety)

```text
Found 161 errors in 9 files
```

- Most errors: "Returning Any from function declared to return X"
- Root cause: Transport layer returns `Any`, resources don't cast
- **Decision**: Defer to iteration 06 — requires systematic fix

---

## Prioritized Actions for This Iteration

### Action 1: Fix Ruff Errors ✅ (DO NOW)

**Rationale**: 7 unused import errors, auto-fixable in 5 seconds.

```bash
ruff check --fix edgequake/
```

**Impact**: Clean lint output, professional codebase

### Action 2: Fix E2E Timing Issue (DEFER to iteration 06)

**Rationale**: Requires code change to test file. Better to batch with mypy fixes.

### Action 3: Mypy Type Fixes (DEFER to iteration 06)

**Rationale**: 161 errors require systematic approach:

1. Add `response_type` parameter usage in `_get` methods
2. Or add explicit `cast()` calls
3. Estimated: 30-60 minutes of work

---

## Decision Matrix

| Action            | Iteration | Rationale          |
| ----------------- | --------- | ------------------ |
| Fix ruff errors   | **05**    | Trivial, immediate |
| Verify docs exist | **05**    | Read-only check    |
| Fix mypy errors   | 06        | Systematic work    |
| Fix E2E timing    | 06        | Code change needed |
| Add export test   | 07+       | Low priority       |

---

## Iteration 05 Scope

```text
Scope boundary for OODA-05:
┌────────────────────────────────────────────────┐
│  ✅ IN SCOPE                                    │
│  • Fix 7 ruff errors (auto-fix)                │
│  • Verify documentation exists                 │
│  • Document mypy findings for iteration 06     │
│  • Commit OODA-05 artifacts                    │
├────────────────────────────────────────────────┤
│  ❌ OUT OF SCOPE                               │
│  • Fix 161 mypy errors (iteration 06)          │
│  • Fix E2E timing issue (iteration 06)         │
│  • Write new documentation (iteration 07+)     │
└────────────────────────────────────────────────┘
```

---

## Success Criteria for ACT Phase

1. ✅ `ruff check edgequake/` returns 0 errors
2. ✅ Documentation verified (exists or gap documented)
3. ✅ OODA files created and committed
4. ⚠️ Mypy errors documented (to be fixed in iteration 06)

---

## Commit Plan

```text
OODA-05: Python SDK lint cleanup and Phase 2 audit

- Fixed 7 ruff errors (unused imports)
- Documented 161 mypy type errors for iteration 06
- Verified 96.9% E2E pass rate (31/32 tests)
- Confirmed 100% lineage endpoint coverage
```

---

## Carryover to Iteration 06

```markdown
## Iteration 06 Focus: Python SDK Type Safety

### Deferred Work

1. Fix 161 mypy errors (Returning Any)
2. Fix E2E test timing issue (retry loop)
3. Add export_lineage E2E test

### Approach for Mypy Fixes

Option A: Add `cast()` at return sites (161 changes)
Option B: Fix `_get()` method signature (1 change + type inference)
→ Recommend Option B for DRY principle
```
