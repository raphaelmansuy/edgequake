# Prioritization & Roadmap

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Execution phases and task prioritization

---

## Table of Contents

1. [Prioritization Framework](#prioritization-framework)
2. [Priority Matrix](#priority-matrix)
3. [Phase Overview](#phase-overview)
4. [Phase 1: Foundation](#phase-1-foundation)
5. [Phase 2: Core Features](#phase-2-core-features)
6. [Phase 3: Advanced Features](#phase-3-advanced-features)
7. [Phase 4: Polish & QA](#phase-4-polish--qa)
8. [Timeline Summary](#timeline-summary)
9. [Risk Mitigation](#risk-mitigation)

---

## Prioritization Framework

### Scoring Criteria

Each gap is scored on three dimensions:

| Dimension  | Weight | Description                                         |
| ---------- | ------ | --------------------------------------------------- |
| **Impact** | 40%    | Business value, user benefit, competitive necessity |
| **Effort** | 30%    | Development time, complexity, dependencies          |
| **Risk**   | 30%    | Technical risk, dependencies, blockers              |

### Scoring Scale

- **Impact:** 1 (Low) to 5 (Critical)
- **Effort:** 1 (XL - 1+ week) to 5 (XS - < 2 hours)
- **Risk:** 1 (High risk) to 5 (Low risk)

### Priority Formula

```
Priority Score = (Impact × 0.4) + (Effort × 0.3) + (Risk × 0.3)
```

---

## Priority Matrix

| Gap ID  | Feature          | Impact | Effort | Risk | Score   | Priority |
| ------- | ---------------- | ------ | ------ | ---- | ------- | -------- |
| GAP-001 | i18n             | 5      | 2      | 4    | **3.8** | P1       |
| GAP-005 | Doc Pagination   | 5      | 4      | 5    | **4.7** | P1       |
| GAP-006 | Doc Filtering    | 5      | 4      | 5    | **4.7** | P1       |
| GAP-007 | Pipeline Status  | 4      | 3      | 4    | **3.7** | P1       |
| GAP-002 | Node Drag        | 4      | 4      | 5    | **4.3** | P1       |
| GAP-008 | LaTeX            | 4      | 3      | 4    | **3.7** | P2       |
| GAP-009 | Mermaid          | 4      | 3      | 4    | **3.7** | P2       |
| GAP-010 | COT Display      | 4      | 3      | 4    | **3.7** | P2       |
| GAP-003 | Graph Layouts    | 3      | 3      | 5    | **3.6** | P2       |
| GAP-004 | Graph Search     | 4      | 3      | 4    | **3.7** | P2       |
| GAP-011 | Syntax Highlight | 3      | 4      | 5    | **3.9** | P2       |
| GAP-012 | Entity Editing   | 3      | 2      | 3    | **2.7** | P3       |
| GAP-013 | Entity Merge     | 3      | 3      | 3    | **3.0** | P3       |
| GAP-014 | Prompt History   | 3      | 4      | 5    | **3.9** | P2       |
| GAP-015 | Search History   | 3      | 4      | 5    | **3.9** | P3       |
| GAP-016 | Frontend Tests   | 4      | 1      | 3    | **2.8** | P3       |
| GAP-017 | Query Prefix     | 2      | 5      | 5    | **3.8** | P3       |
| GAP-018 | Tab Visibility   | 2      | 4      | 5    | **3.5** | P4       |
| GAP-019 | Full-screen      | 2      | 5      | 5    | **3.8** | P4       |
| GAP-020 | Graph Legend     | 2      | 4      | 5    | **3.5** | P4       |

---

## Phase Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     IMPLEMENTATION TIMELINE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: Foundation (Week 1-2)                                  │
│  ├── i18n Setup                                                  │
│  ├── Document Pagination                                         │
│  ├── Document Filtering                                          │
│  └── Pipeline Status                                             │
│                                                                  │
│  Phase 2: Core Features (Week 3-4)                               │
│  ├── Node Drag & Drop                                            │
│  ├── LaTeX Rendering                                             │
│  ├── Mermaid Diagrams                                            │
│  ├── COT/Thinking Display                                        │
│  └── Graph Search                                                │
│                                                                  │
│  Phase 3: Advanced Features (Week 5-6)                           │
│  ├── Graph Layouts                                               │
│  ├── Syntax Highlighting                                         │
│  ├── User Prompt History                                         │
│  ├── Entity Editing                                              │
│  └── Entity Merge                                                │
│                                                                  │
│  Phase 4: Polish & QA (Week 7-8)                                 │
│  ├── Frontend Tests                                              │
│  ├── Tab Visibility                                              │
│  ├── Full-screen Mode                                            │
│  ├── Graph Legend                                                │
│  └── Bug Fixes & Refinement                                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Foundation

> **Duration:** 2 weeks  
> **Focus:** Critical infrastructure and high-impact document features

### Week 1: Core Infrastructure

#### Task 1.1: Internationalization Setup

**Gap:** GAP-001  
**Duration:** 3 days  
**Dependencies:** None

| Day | Tasks                                                |
| --- | ---------------------------------------------------- |
| 1   | Install i18next, create i18n config, set up provider |
| 2   | Create en.json locale file with all translation keys |
| 3   | Add language selector, integrate with settings store |

**Deliverables:**

- [x] i18n configuration
- [x] English locale file (complete)
- [x] Language selector component
- [x] All components using `t()` function

**Acceptance Criteria:**

- Language can be switched dynamically
- All UI text comes from locale files
- Language preference persists across sessions

---

#### Task 1.2: Document Pagination

**Gap:** GAP-005  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                                 | Hours |
| --------------------------------------- | ----- |
| Create PaginationControls component     | 2     |
| Implement useUrlState hook              | 2     |
| Integrate with document manager         | 2     |
| Update API calls with pagination params | 2     |

**Deliverables:**

- [x] PaginationControls component
- [x] useUrlState hook
- [x] URL state synchronization
- [x] Updated getDocuments API

---

#### Task 1.3: Document Filtering

**Gap:** GAP-006  
**Duration:** 0.5 day  
**Dependencies:** Task 1.2

| Subtask                          | Hours |
| -------------------------------- | ----- |
| Create DocumentFilters component | 2     |
| Add filter state to URL          | 1     |
| Integrate with document table    | 1     |

---

### Week 2: Document Management Enhancement

#### Task 1.4: Pipeline Status Monitoring

**Gap:** GAP-007  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                               | Hours |
| ------------------------------------- | ----- |
| Create PipelineStatusDialog component | 3     |
| Add pipeline status API endpoint      | 2     |
| Add cancel pipeline functionality     | 2     |
| Add busy indicator to header          | 1     |

**Deliverables:**

- [x] PipelineStatusDialog component
- [x] Pipeline status API integration
- [x] Cancel pipeline functionality
- [x] Visual busy indicator

---

#### Task 1.5: Create Chinese and French Locales

**Gap:** GAP-001 (continuation)  
**Duration:** 2 days  
**Dependencies:** Task 1.1

| Day | Tasks                           |
| --- | ------------------------------- |
| 1   | Create zh.json (Chinese) locale |
| 2   | Create fr.json (French) locale  |

---

#### Task 1.6: Phase 1 Integration Testing

**Duration:** 1 day  
**Dependencies:** All Phase 1 tasks

| Subtask                      | Hours |
| ---------------------------- | ----- |
| Test pagination with filters | 2     |
| Test language switching      | 2     |
| Test pipeline monitoring     | 2     |
| Fix integration issues       | 2     |

---

### Phase 1 Deliverables Summary

| Deliverable                  | Status |
| ---------------------------- | ------ |
| i18n with 3 languages        | 🔴     |
| Document pagination          | 🔴     |
| Document filtering & sorting | 🔴     |
| Pipeline status dialog       | 🔴     |
| URL state sync               | 🔴     |

---

## Phase 2: Core Features

> **Duration:** 2 weeks  
> **Focus:** Graph enhancements and query improvements

### Week 3: Graph Visualization

#### Task 2.1: Node Drag & Drop

**Gap:** GAP-002  
**Duration:** 0.5 day  
**Dependencies:** None

| Subtask                      | Hours |
| ---------------------------- | ----- |
| Create GraphEvents component | 2     |
| Integrate with GraphRenderer | 1     |
| Add enable/disable setting   | 1     |

---

#### Task 2.2: Graph Node Search

**Gap:** GAP-004  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                      | Hours |
| ---------------------------- | ----- |
| Install MiniSearch           | 0.5   |
| Create GraphSearch component | 4     |
| Add camera focus animation   | 2     |
| Integrate with toolbar       | 1.5   |

---

#### Task 2.3: LaTeX Rendering

**Gap:** GAP-008  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                           | Hours |
| --------------------------------- | ----- |
| Install KaTeX and plugins         | 1     |
| Create MarkdownRenderer component | 4     |
| Handle dynamic loading            | 2     |
| Test math formulas                | 1     |

---

### Week 4: Query Enhancements

#### Task 2.4: Mermaid Diagrams

**Gap:** GAP-009  
**Duration:** 1 day  
**Dependencies:** Task 2.3

| Subtask                         | Hours |
| ------------------------------- | ----- |
| Install Mermaid                 | 0.5   |
| Create MermaidDiagram component | 4     |
| Integrate with MarkdownRenderer | 2     |
| Add theme awareness             | 1.5   |

---

#### Task 2.5: COT/Thinking Display

**Gap:** GAP-010  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                          | Hours |
| -------------------------------- | ----- |
| Create parseCOTContent utility   | 2     |
| Create ThinkingDisplay component | 3     |
| Integrate with ChatMessage       | 2     |
| Add thinking time tracking       | 1     |

---

#### Task 2.6: Enhanced Syntax Highlighting

**Gap:** GAP-011  
**Duration:** 0.5 day  
**Dependencies:** Task 2.3

| Subtask                          | Hours |
| -------------------------------- | ----- |
| Install react-syntax-highlighter | 0.5   |
| Create CodeHighlight component   | 2     |
| Add theme switching              | 1.5   |

---

#### Task 2.7: Phase 2 Integration Testing

**Duration:** 1 day  
**Dependencies:** All Phase 2 tasks

---

### Phase 2 Deliverables Summary

| Deliverable         | Status |
| ------------------- | ------ |
| Node drag & drop    | 🔴     |
| Graph node search   | 🔴     |
| LaTeX rendering     | 🔴     |
| Mermaid diagrams    | 🔴     |
| COT display         | 🔴     |
| Syntax highlighting | 🔴     |

---

## Phase 3: Advanced Features

> **Duration:** 2 weeks  
> **Focus:** Graph layouts, entity management, history features

### Week 5: Graph Advanced Features

#### Task 3.1: Graph Layout Algorithms

**Gap:** GAP-003  
**Duration:** 1 day  
**Dependencies:** None

| Subtask                           | Hours |
| --------------------------------- | ----- |
| Install layout packages           | 1     |
| Create LayoutControl component    | 4     |
| Integrate with graph viewer       | 2     |
| Add layout preference persistence | 1     |

---

#### Task 3.2: User Prompt History UI

**Gap:** GAP-014  
**Duration:** 0.5 day  
**Dependencies:** None

| Subtask                      | Hours |
| ---------------------------- | ----- |
| Create PromptHistoryDropdown | 3     |
| Add keyboard navigation      | 1     |

---

#### Task 3.3: Search History Manager

**Gap:** GAP-015  
**Duration:** 0.5 day  
**Dependencies:** None

| Subtask                             | Hours |
| ----------------------------------- | ----- |
| Create SearchHistoryManager utility | 3     |
| Integrate with GraphSearch          | 1     |

---

### Week 6: Entity Management

#### Task 3.4: Entity Property Editing

**Gap:** GAP-012  
**Duration:** 2 days  
**Dependencies:** API support required

| Day | Tasks                                           |
| --- | ----------------------------------------------- |
| 1   | Create PropertyEditDialog, EditablePropertyRow  |
| 2   | Add API integration, validation, error handling |

---

#### Task 3.5: Entity Merge

**Gap:** GAP-013  
**Duration:** 1 day  
**Dependencies:** Task 3.4, API support required

| Subtask                      | Hours |
| ---------------------------- | ----- |
| Create MergeDialog component | 3     |
| Add merge API integration    | 3     |
| Add post-merge refresh logic | 2     |

---

#### Task 3.6: Query Mode Prefix

**Gap:** GAP-017  
**Duration:** 0.5 day  
**Dependencies:** None

| Subtask                                | Hours |
| -------------------------------------- | ----- |
| Add prefix parsing to query submission | 2     |
| Add error messages for invalid modes   | 1     |
| Update UI to show active mode          | 1     |

---

### Phase 3 Deliverables Summary

| Deliverable                | Status |
| -------------------------- | ------ |
| Multiple graph layouts     | 🔴     |
| User prompt history UI     | 🔴     |
| Search history persistence | 🔴     |
| Entity property editing    | 🔴     |
| Entity merge functionality | 🔴     |
| Query mode prefix          | 🔴     |

---

## Phase 4: Polish & QA

> **Duration:** 2 weeks  
> **Focus:** Testing, optimization, and final polish

### Week 7: Testing Infrastructure

#### Task 4.1: Frontend Test Setup

**Gap:** GAP-016  
**Duration:** 2 days  
**Dependencies:** None

| Day | Tasks                                             |
| --- | ------------------------------------------------- |
| 1   | Set up Vitest/Jest, configure testing environment |
| 2   | Create test utilities, mock providers             |

---

#### Task 4.2: Write Core Tests

**Duration:** 3 days  
**Dependencies:** Task 4.1

| Day | Focus Area                          |
| --- | ----------------------------------- |
| 1   | Store tests (auth, settings, graph) |
| 2   | Component tests (documents, query)  |
| 3   | Hook tests, integration tests       |

---

### Week 8: Polish & Optimization

#### Task 4.3: Tab Visibility Optimization

**Gap:** GAP-018  
**Duration:** 0.5 day

---

#### Task 4.4: Full-Screen Mode

**Gap:** GAP-019  
**Duration:** 0.5 day

---

#### Task 4.5: Graph Legend

**Gap:** GAP-020  
**Duration:** 0.5 day

---

#### Task 4.6: Bug Fixes & Refinement

**Duration:** 3 days

---

### Phase 4 Deliverables Summary

| Deliverable                 | Status |
| --------------------------- | ------ |
| Test infrastructure         | 🔴     |
| Core test suite (50+ tests) | 🔴     |
| Tab visibility optimization | 🔴     |
| Full-screen mode            | 🔴     |
| Graph legend                | 🔴     |
| Bug fixes                   | 🔴     |

---

## Timeline Summary

```
┌──────────────────────────────────────────────────────────────────┐
│                        8-WEEK TIMELINE                           │
├──────────────────────────────────────────────────────────────────┤
│ Week 1-2:  Phase 1 - Foundation                                  │
│            • i18n setup and locales                              │
│            • Document pagination & filtering                     │
│            • Pipeline status monitoring                          │
├──────────────────────────────────────────────────────────────────┤
│ Week 3-4:  Phase 2 - Core Features                               │
│            • Graph enhancements (drag, search)                   │
│            • Query improvements (LaTeX, Mermaid, COT)            │
├──────────────────────────────────────────────────────────────────┤
│ Week 5-6:  Phase 3 - Advanced Features                           │
│            • Graph layouts                                       │
│            • Entity management                                   │
│            • History features                                    │
├──────────────────────────────────────────────────────────────────┤
│ Week 7-8:  Phase 4 - Polish & QA                                 │
│            • Testing infrastructure                              │
│            • Core test suite                                     │
│            • Bug fixes and optimization                          │
└──────────────────────────────────────────────────────────────────┘
```

### Milestones

| Milestone | Target Date   | Key Deliverables                             |
| --------- | ------------- | -------------------------------------------- |
| M1        | End of Week 2 | i18n, pagination, filtering, pipeline status |
| M2        | End of Week 4 | Graph interactions, LaTeX, Mermaid, COT      |
| M3        | End of Week 6 | Entity management, history, layouts          |
| M4        | End of Week 8 | Full test coverage, production ready         |

---

## Risk Mitigation

### Identified Risks

| Risk                     | Impact | Probability | Mitigation                                     |
| ------------------------ | ------ | ----------- | ---------------------------------------------- |
| API changes required     | High   | Medium      | Coordinate with backend team early             |
| i18n translation quality | Medium | Medium      | Use professional translation for key languages |
| Mermaid performance      | Medium | Low         | Lazy load, render on demand                    |
| Test coverage time       | Medium | High        | Focus on critical paths first                  |
| Browser compatibility    | Low    | Low         | Test on major browsers early                   |

### Contingency Plan

If timeline slips:

1. **Week 1-2 delay:** Reduce locales to English only initially
2. **Week 3-4 delay:** Move Mermaid to Phase 3
3. **Week 5-6 delay:** Defer entity editing to post-launch
4. **Week 7-8 delay:** Ship with minimal tests, add post-launch

### Dependencies

| Phase   | External Dependencies                |
| ------- | ------------------------------------ |
| Phase 1 | None                                 |
| Phase 2 | None                                 |
| Phase 3 | Backend API for entity editing/merge |
| Phase 4 | CI/CD pipeline for test automation   |

---

## Cross-References

| Document                                          | Relationship            |
| ------------------------------------------------- | ----------------------- |
| [Gap Analysis](./002-gap-analysis.md)             | Source of requirements  |
| [Proposed Solutions](./003-proposed-solutions.md) | Implementation details  |
| [UX Improvements](./005-ux-improvements.md)       | UX-specific priorities  |
| [Success Criteria](./008-success-criteria.md)     | Milestone validation    |
| [Developer Guide](./009-developer-guide.md)       | Implementation guidance |

---

_Document defines execution order and timeline for gap remediation_
