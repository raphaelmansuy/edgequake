# Prioritization & Roadmap

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Execution phases with prioritized tasks

---

## Table of Contents

1. [Prioritization Framework](#prioritization-framework)
2. [Priority Matrix](#priority-matrix)
3. [Execution Phases](#execution-phases)
4. [Sprint Breakdown](#sprint-breakdown)
5. [Dependencies & Risks](#dependencies--risks)
6. [Cross-References](#cross-references)

---

## Prioritization Framework

### Scoring Criteria

| Factor     | Weight | Description                                        |
| ---------- | ------ | -------------------------------------------------- |
| **Impact** | 40%    | User experience improvement, feature completeness  |
| **Effort** | 30%    | Development time, complexity, testing requirements |
| **Risk**   | 20%    | Technical risk, regression potential               |
| **Deps**   | 10%    | Dependency on other tasks or external factors      |

### Scoring Scale

- **Impact:** 1 (Low) to 5 (Critical)
- **Effort:** 1 (Trivial) to 5 (Complex)
- **Risk:** 1 (Low) to 5 (High)
- **Priority Score:** (Impact × 0.4) + ((6-Effort) × 0.3) + ((6-Risk) × 0.2) + ((6-Deps) × 0.1)

---

## Priority Matrix

| Gap ID  | Feature                    | Impact | Effort | Risk | Deps | Score | Priority   |
| ------- | -------------------------- | ------ | ------ | ---- | ---- | ----- | ---------- |
| GAP-001 | Node Drag & Drop           | 5      | 2      | 2    | 1    | 4.3   | 🔴 Phase 1 |
| GAP-002 | Multiple Layout Algorithms | 4      | 3      | 2    | 2    | 3.6   | 🔴 Phase 1 |
| GAP-003 | Fuzzy Node Search          | 4      | 2      | 1    | 1    | 4.0   | 🔴 Phase 1 |
| GAP-004 | Pipeline Status Dialog     | 4      | 3      | 2    | 1    | 3.7   | 🔴 Phase 1 |
| GAP-005 | Translation Coverage       | 3      | 3      | 1    | 1    | 3.4   | 🟡 Phase 2 |
| GAP-006 | URL State Sync             | 3      | 2      | 1    | 1    | 3.6   | 🟡 Phase 2 |
| GAP-007 | Query Mode Prefix          | 3      | 1      | 1    | 1    | 3.8   | 🟡 Phase 2 |
| GAP-008 | Thinking Time Display      | 3      | 2      | 1    | 1    | 3.6   | 🟡 Phase 2 |
| GAP-009 | Full-Screen Graph          | 2      | 1      | 1    | 1    | 3.4   | 🟡 Phase 2 |
| GAP-010 | Entity Merge               | 3      | 3      | 2    | 2    | 3.1   | 🟡 Phase 2 |
| GAP-011 | Graph Legend               | 2      | 2      | 1    | 1    | 3.2   | 🟢 Phase 3 |
| GAP-012 | Graph Settings Panel       | 2      | 3      | 1    | 3    | 2.8   | 🟢 Phase 3 |
| GAP-013 | Inline Property Edit       | 2      | 4      | 3    | 2    | 2.3   | 🟢 Phase 3 |
| GAP-014 | User Prompt History        | 2      | 2      | 1    | 1    | 3.2   | 🟢 Phase 3 |
| GAP-015 | Copy Response              | 2      | 1      | 1    | 1    | 3.4   | 🟢 Phase 3 |

---

## Execution Phases

### Phase 1: Core Graph Enhancements 🔴

**Duration:** 2 weeks (10 working days)  
**Goal:** Achieve feature parity for graph visualization

| Task                       | Days | Owner | Dependencies |
| -------------------------- | ---- | ----- | ------------ |
| Node Drag & Drop           | 2    | FE    | None         |
| Multiple Layout Algorithms | 2    | FE    | None         |
| Fuzzy Node Search          | 2    | FE    | None         |
| Pipeline Status Dialog     | 2    | FE    | API ready    |
| Integration Testing        | 2    | QA    | All above    |

**Deliverables:**

- Interactive graph with drag-and-drop nodes
- 5+ layout algorithm options
- Fast fuzzy search across all entities
- Detailed pipeline status monitoring

---

### Phase 2: Query & Document UX 🟡

**Duration:** 2 weeks (10 working days)  
**Goal:** Enhanced query experience and document management

| Task                  | Days | Owner | Dependencies |
| --------------------- | ---- | ----- | ------------ |
| Translation Coverage  | 2    | FE    | None         |
| URL State Sync        | 2    | FE    | None         |
| Query Mode Prefix     | 1    | FE    | None         |
| Thinking Time Display | 1    | FE    | None         |
| Full-Screen Graph     | 1    | FE    | Phase 1      |
| Entity Merge Dialog   | 2    | FE    | API ready    |
| Integration Testing   | 1    | QA    | All above    |

**Deliverables:**

- Complete i18n coverage (en, zh, fr)
- Shareable URLs with state
- Power-user query shortcuts
- Transparent thinking time
- Full-screen presentation mode
- Entity deduplication workflow

---

### Phase 3: Polish & Refinement 🟢

**Duration:** 1 week (5 working days)  
**Goal:** Final touches and edge cases

| Task                      | Days | Owner | Dependencies |
| ------------------------- | ---- | ----- | ------------ |
| Graph Legend              | 1    | FE    | Phase 1      |
| Graph Settings Panel      | 1    | FE    | Phase 1      |
| User Prompt History       | 1    | FE    | None         |
| Copy Response Button      | 0.5  | FE    | None         |
| Inline Property Edit      | 1    | FE    | Phase 1      |
| Final Testing & Bug Fixes | 0.5  | QA    | All above    |

**Deliverables:**

- Visual legend for graph colors
- Configurable graph settings
- Query prompt templates
- Easy response sharing
- In-place entity editing

---

## Sprint Breakdown

### Sprint 1 (Week 1)

**Focus:** Graph Interactivity

```
Day 1-2: Node Drag & Drop
├── Create GraphEvents component
├── Add drag state to store
├── Test with large graphs
└── Handle edge cases (zoom, pan)

Day 3-4: Multiple Layouts
├── Install layout packages
├── Create LayoutControl dropdown
├── Add layout persistence
└── Test all 5 layouts

Day 5: Buffer / Code Review
```

### Sprint 2 (Week 2)

**Focus:** Search & Pipeline

```
Day 1-2: Fuzzy Search
├── Implement MiniSearch integration
├── Create search popover
├── Add keyboard navigation
└── Camera focus on selection

Day 3-4: Pipeline Status
├── Enhance dialog component
├── Add progress details
├── Implement cancellation
└── Auto-refresh logic

Day 5: Integration Testing
```

### Sprint 3 (Week 3)

**Focus:** i18n & URLs

```
Day 1-2: Translation Coverage
├── Add ~350 translation keys
├── Review all components
├── Test language switching
└── Verify RTL-ready structure

Day 3-4: URL State Sync
├── Create useUrlState hook
├── Integrate with DocumentManager
├── Add to query page
└── Test browser navigation
```

### Sprint 4 (Week 4)

**Focus:** Query Enhancements

```
Day 1: Query Mode Prefix
├── Add prefix parsing
├── Update placeholder text
├── Add error handling
└── Test all modes

Day 2: Thinking Time + Full-Screen
├── Track thinking duration
├── Display in message
├── Add fullscreen control
└── Test keyboard shortcuts

Day 3-4: Entity Merge
├── Create merge dialog
├── Add API integration
├── Handle conflicts
└── Test refresh after merge

Day 5: Integration Testing
```

### Sprint 5 (Week 5)

**Focus:** Polish

```
Day 1: Graph Legend
├── Create Legend component
├── Add toggle button
├── Generate from data
└── Style for themes

Day 2: Graph Settings
├── Create settings panel
├── Persist preferences
├── Add reset option
└── Document options

Day 3: Prompt History + Copy
├── Add prompt history store
├── Create history dropdown
├── Add copy button
└── Toast confirmation

Day 4: Inline Property Edit
├── Create editable row
├── Add save/cancel
├── Handle validation
└── Test updates

Day 5: Final QA
├── Run full test suite
├── Performance testing
├── Accessibility audit
└── Documentation review
```

---

## Dependencies & Risks

### External Dependencies

| Dependency                | Owner   | Status   | Mitigation                  |
| ------------------------- | ------- | -------- | --------------------------- |
| Entity merge API endpoint | Backend | ✅ Ready | Verify response format      |
| Pipeline cancel endpoint  | Backend | ⚠️ Check | Implement without if needed |
| Document scan endpoint    | Backend | ✅ Ready | N/A                         |

### Technical Risks

| Risk                       | Probability | Impact | Mitigation                         |
| -------------------------- | ----------- | ------ | ---------------------------------- |
| Sigma.js version conflicts | Low         | High   | Pin versions, test thoroughly      |
| Large graph performance    | Medium      | Medium | Implement virtualization if needed |
| i18n key conflicts         | Low         | Low    | Namespace keys properly            |
| SSR issues with graph      | Low         | Medium | Already using dynamic imports      |

### Rollback Plan

If a phase introduces critical bugs:

1. Revert to previous stable commit
2. Create hotfix branch
3. Fix issues in isolation
4. Re-test before merge
5. Deploy incrementally

---

## Success Milestones

### Phase 1 Complete ✓

- [ ] All graph tests pass
- [ ] Performance < 100ms for 1000 nodes
- [ ] No regressions in existing features

### Phase 2 Complete ✓

- [ ] 100% translation coverage
- [ ] All URLs shareable
- [ ] Query shortcuts documented

### Phase 3 Complete ✓

- [ ] All E2E tests pass
- [ ] Lighthouse score > 90
- [ ] User acceptance sign-off

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md)
- **Proposed Solutions:** [002-proposed-solutions.md](./002-proposed-solutions.md)
- **UX Plan:** [005-ux-improvements.md](./005-ux-improvements.md)
- **QA Plan:** [007-qa-plan.md](./007-qa-plan.md)
- **Success Criteria:** [008-success-criteria.md](./008-success-criteria.md)
