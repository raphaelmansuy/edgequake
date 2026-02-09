# OODA Iteration 03 - Decide

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Decision Summary

Based on the Orient analysis, prioritize actions by signal value:

### High Priority (This Iteration)

1. **Run E2E tests to verify all fixes work** - HIGH signal
   - This validates iterations 01 & 02 actually fixed the issues
2. **Benchmark loading time** - HIGH signal
   - Need data to confirm <2s target met

### Medium Priority (Next Iterations)

3. **Add basic keyboard navigation** - MEDIUM signal
   - Down to select, Enter to expand, Escape to deselect
   - Improves accessibility

4. **Add aria-live announcements** - MEDIUM signal
   - Screen readers need to know about graph changes

### Low Priority (Future)

5. **Backend 500 cap** - LOW signal
   - Frontend already enforces; defense in depth
6. **Color contrast validation** - LOW signal
   - Type colors are standard palette, likely OK

---

## Specific Changes Decided

### Change 1: Run TypeScript Tests

**Command**: `pnpm test`
**Verify**: Graph components have passing tests

### Change 2: Benchmark Graph Loading

**Tool**: Browser DevTools Performance tab
**Metric**: Time from fetch start to render complete
**Target**: <2000ms for 500 nodes

### Change 3: Run Backend Tests

**Command**: `cargo test -p edgequake-api`
**Verify**: Entity neighborhood handler tests pass

---

## Acceptance Criteria

- [ ] TypeScript tests pass
- [ ] Rust API tests pass
- [ ] Loading time benchmark documented
- [ ] Graph settings panel shows max=500
- [ ] Labels visible at default zoom

---

## Dependency Check

```
Test Suite Dependencies:
├── Frontend (pnpm test)
│   ├── Jest + Testing Library
│   └── Component tests for graph
│
├── Backend (cargo test)
│   ├── Tokio test runtime
│   └── Handler tests for entities.rs
│
└── E2E (Playwright)
    └── Browser automation
```

---

## Next Iteration Plan

After this iteration:

- Iteration 04: Add keyboard navigation
- Iteration 05: Add aria-live regions
- Iteration 06-10: Systematic WCAG compliance
