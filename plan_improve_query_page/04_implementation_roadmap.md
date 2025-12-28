# Phase 4: Implementation Roadmap - Query Page UX/UI Improvement

> **Date**: December 27, 2025  
> **Dependencies**: [Technical Spec](./03_technical_spec.md), [Design Strategy](./02_design_strategy.md)  
> **Timeline**: 3 Sprint Cycles (6 weeks)

---

## 1. Prioritization Matrix

### 1.1 Priority Definitions

| Priority | Label    | Definition                                                | Timeline   |
| -------- | -------- | --------------------------------------------------------- | ---------- |
| **P0**   | Critical | Blocking production use, security risk, or data integrity | Sprint 1   |
| **P1**   | High     | Significant UX degradation, frequently encountered issues | Sprint 1-2 |
| **P2**   | Medium   | Notable improvement, quality of life enhancement          | Sprint 2-3 |
| **P3**   | Low      | Nice-to-have, polish, future consideration                | Backlog    |

### 1.2 Feature Prioritization

| ID         | Feature                         | Priority | Effort | Dependencies | User Impact                       |
| ---------- | ------------------------------- | -------- | ------ | ------------ | --------------------------------- |
| **MD-01**  | Table streaming buffering       | P0       | M      | None         | 🔴 Tables break during generation |
| **MD-02**  | DOMPurify HTML sanitization     | P0       | S      | None         | 🔴 Security vulnerability         |
| **MD-03**  | Auto-scroll optimization        | P1       | S      | None         | 🟠 Scroll jank during streaming   |
| **MD-04**  | GitHub-style alerts extension   | P1       | M      | MD-02        | 🟡 Missing standard markdown      |
| **MD-05**  | Footnotes extension             | P2       | M      | MD-02        | 🟡 Academic content support       |
| **MD-06**  | Enhanced citations with preview | P2       | L      | API changes  | 🟡 Source discoverability         |
| **MD-07**  | Collapsible details blocks      | P3       | S      | MD-04        | 🟢 Long response handling         |
| **DB-01**  | Materialized message counts     | P1       | M      | Migration    | 🟠 List query performance         |
| **DB-02**  | Message versioning table        | P2       | M      | Migration    | 🟡 Edit history tracking          |
| **DB-03**  | Conversation tags               | P3       | M      | Migration    | 🟢 Organization feature           |
| **API-01** | Message pagination endpoint     | P1       | M      | DB-01        | 🟠 Long conversation support      |
| **API-02** | Enhanced conversation search    | P2       | M      | DB-01        | 🟡 Find past queries faster       |
| **UI-01**  | Streaming text animations       | P2       | S      | MD-03        | 🟡 Visual polish                  |
| **UI-02**  | Thinking section animations     | P2       | S      | None         | 🟡 Visual polish                  |
| **UI-03**  | Empty state enhancement         | P2       | S      | None         | 🟡 Onboarding improvement         |
| **UI-04**  | Mobile history panel sheet      | P1       | M      | None         | 🟠 Mobile usability               |
| **UI-05**  | Conversation filters UI         | P1       | M      | API-02       | 🟠 Find conversations             |
| **UI-06**  | Keyboard shortcuts              | P3       | S      | None         | 🟢 Power user feature             |

### 1.3 Dependency Graph

```
                                    ┌─────────────────┐
                                    │    Sprint 1     │
                                    └────────┬────────┘
                                             │
           ┌─────────────────────────────────┼─────────────────────────────────┐
           │                                 │                                 │
           ▼                                 ▼                                 ▼
    ┌──────────────┐                 ┌──────────────┐                 ┌──────────────┐
    │    MD-01     │                 │    MD-02     │                 │    DB-01     │
    │   Table      │                 │  DOMPurify   │                 │  Materialized│
    │  Buffering   │                 │ Sanitization │                 │   Counts     │
    └──────────────┘                 └──────┬───────┘                 └──────┬───────┘
                                            │                                 │
                                            │                                 │
                                    ┌───────┴───────┐                         │
                                    │               │                         │
                                    ▼               ▼                         ▼
                             ┌──────────────┐ ┌──────────────┐         ┌──────────────┐
                             │    MD-04     │ │    MD-05     │         │   API-01     │
                             │   Alerts     │ │  Footnotes   │         │  Pagination  │
                             └──────────────┘ └──────────────┘         └──────────────┘
                                    │                                         │
                                    │                                         │
                                    ▼                                         ▼
                             ┌──────────────┐                         ┌──────────────┐
                             │    MD-07     │                         │   API-02     │
                             │   Details    │                         │   Search     │
                             └──────────────┘                         └──────────────┘
                                                                              │
                                                                              │
                                                                              ▼
                                                                       ┌──────────────┐
                                                                       │    UI-05     │
                                                                       │   Filters    │
                                                                       └──────────────┘
```

---

## 2. Sprint-by-Sprint Plan

### Sprint 1: Foundation & Critical Fixes (2 weeks)

**Theme**: Fix breaking issues, establish performance baseline

#### Week 1: Core Fixes

| Task                                       | Assignee | Days | Acceptance Criteria                                              |
| ------------------------------------------ | -------- | ---- | ---------------------------------------------------------------- |
| **MD-01**: Implement table buffering logic | Frontend | 2    | Tables render only when complete; show skeleton during buffering |
| **MD-02**: Integrate DOMPurify             | Frontend | 1    | All HTML tokens sanitized; XSS test suite passes                 |
| **MD-03**: Optimize auto-scroll            | Frontend | 1    | 60fps scroll during streaming; user scroll disables auto-scroll  |
| **DB-01**: Add message_count migration     | Backend  | 1    | Migration runs in <30s on 10k conversations                      |

#### Week 2: UI Foundations

| Task                            | Assignee | Days | Acceptance Criteria                                                     |
| ------------------------------- | -------- | ---- | ----------------------------------------------------------------------- |
| **UI-04**: Mobile history sheet | Frontend | 3    | Sheet slides from left; touch gestures work; < 200ms open time          |
| **API-01**: Message pagination  | Backend  | 2    | Cursor-based pagination; returns 50 messages default; includes has_more |

#### Sprint 1 Deliverables Checklist

- [ ] Tables render correctly during streaming (no broken HTML)
- [ ] All HTML content sanitized via DOMPurify
- [ ] Auto-scroll smooth at 60fps
- [ ] Mobile users can access conversation history
- [ ] Long conversations paginate properly
- [ ] Database includes message_count column with backfill

#### Sprint 1 Demo Script

1. Start streaming a query with table → Table appears only when complete ✓
2. Inject XSS payload `<img onerror="alert('xss')">` → Sanitized, no alert ✓
3. Stream 500+ tokens → No scroll stutter ✓
4. Open on mobile → Swipe left for history ✓
5. Load conversation with 100+ messages → Only 50 loaded initially ✓

---

### Sprint 2: Markdown Extensions & Polish (2 weeks)

**Theme**: Feature parity with OpenWebUI, visual refinement

#### Week 3: Markdown Extensions

| Task                                 | Assignee | Days | Acceptance Criteria                                                 |
| ------------------------------------ | -------- | ---- | ------------------------------------------------------------------- |
| **MD-04**: GitHub alerts extension   | Frontend | 2    | NOTE, TIP, WARNING, CAUTION, IMPORTANT render with icons and colors |
| **MD-05**: Footnotes extension       | Frontend | 2    | `[^1]` renders as superscript; definitions render at bottom         |
| **UI-01**: Streaming text animations | Frontend | 1    | Tokens fade in with 100ms stagger; cursor pulses smoothly           |

#### Week 4: Performance & Search

| Task                           | Assignee | Days | Acceptance Criteria                                                           |
| ------------------------------ | -------- | ---- | ----------------------------------------------------------------------------- |
| **API-02**: Enhanced search    | Backend  | 2    | Full-text search in title + content; returns in < 100ms for 10k conversations |
| **UI-05**: Filter/sort UI      | Frontend | 2    | Mode filter chips; date picker; sort dropdown; immediate application          |
| **UI-02**: Thinking animations | Frontend | 1    | Brain icon pulses; shimmer bars animate; smooth collapse transition           |

#### Sprint 2 Deliverables Checklist

- [ ] GitHub-style alerts render with appropriate styling
- [ ] Footnotes appear as superscripts with hover preview
- [ ] Streaming text has visible fade-in animation
- [ ] Thinking section has polish animations
- [ ] Search finds conversations by content
- [ ] Filter by mode/date works with immediate feedback

#### Sprint 2 Demo Script

1. Query returns `> [!WARNING]\n> Be careful!` → Yellow warning box renders ✓
2. Response includes `See footnote[^1]` → Superscript with tooltip ✓
3. Stream response → Watch tokens fade in smoothly ✓
4. Search "entity graph" → Find matching conversations quickly ✓
5. Filter by "hybrid" mode → Only hybrid conversations shown ✓

---

### Sprint 3: Advanced Features & Quality (2 weeks)

**Theme**: Edge cases, documentation, launch preparation

#### Week 5: Advanced Markdown & Versioning

| Task                          | Assignee | Days | Acceptance Criteria                                          |
| ----------------------------- | -------- | ---- | ------------------------------------------------------------ |
| **MD-06**: Enhanced citations | Frontend | 2    | Hover shows source preview; click scrolls to source          |
| **DB-02**: Message versioning | Backend  | 2    | Store initial + edit versions; expose via API                |
| **UI-03**: Empty state polish | Frontend | 1    | Animated icon; graph stats display; suggestion hover effects |

#### Week 6: Final Polish & Testing

| Task              | Assignee | Days | Acceptance Criteria                                    |
| ----------------- | -------- | ---- | ------------------------------------------------------ |
| E2E test suite    | QA       | 2    | 90%+ coverage on query page; all critical paths tested |
| Performance audit | Frontend | 1    | Lighthouse score > 90; Core Web Vitals green           |
| Documentation     | All      | 1    | Updated README; API docs; component storybook          |
| Bug bash & fixes  | All      | 1    | All P0/P1 bugs resolved; P2 triaged for future         |

#### Sprint 3 Deliverables Checklist

- [ ] Citations show rich previews on hover
- [ ] Message edit history accessible
- [ ] Empty state feels premium and helpful
- [ ] E2E tests pass on CI
- [ ] Performance metrics meet targets
- [ ] Documentation complete

#### Sprint 3 Demo Script

1. Hover citation → Preview card shows source content ✓
2. Regenerate message → Original version preserved in history ✓
3. Open fresh page → Beautiful empty state with stats ✓
4. Run full test suite → All green ✓
5. Lighthouse audit → 90+ performance score ✓

---

## 3. Risk Register

### 3.1 Technical Risks

| ID     | Risk                                            | Probability | Impact | Mitigation                                         | Contingency                                    |
| ------ | ----------------------------------------------- | ----------- | ------ | -------------------------------------------------- | ---------------------------------------------- |
| **R1** | Table buffering breaks other markdown elements  | Medium      | High   | Comprehensive test suite; edge case analysis       | Roll back to current behavior; fix iteratively |
| **R2** | DOMPurify bundle size impact (+25kb gzipped)    | Low         | Medium | Dynamic import; load only when HTML tokens present | Use lighter alternative (xss library)          |
| **R3** | Message pagination breaks existing UI state     | Medium      | Medium | Feature flag; A/B test with subset                 | Keep unlimited loading as fallback             |
| **R4** | Search performance degrades with scale          | Medium      | High   | EXPLAIN ANALYZE in CI; monitor query times         | Add Redis cache layer for frequent queries     |
| **R5** | Mobile sheet conflicts with iOS scroll behavior | Medium      | Medium | Test on real devices; Safari-specific fixes        | Use native dialog on iOS                       |

### 3.2 Design Risks

| ID     | Risk                                    | Probability | Impact | Mitigation                                   | Contingency            |
| ------ | --------------------------------------- | ----------- | ------ | -------------------------------------------- | ---------------------- |
| **R6** | Animations distract rather than delight | Medium      | Low    | User testing; toggle option in settings      | Make animations opt-in |
| **R7** | New filter UI takes too much space      | Low         | Medium | Collapsible filter bar; hide when not active | Move to popover        |

### 3.3 Timeline Risks

| ID      | Risk                                       | Probability | Impact | Mitigation                                | Contingency                                        |
| ------- | ------------------------------------------ | ----------- | ------ | ----------------------------------------- | -------------------------------------------------- |
| **R8**  | Backend changes take longer than estimated | Medium      | High   | Start DB work early; parallel development | Defer API-02 to next cycle; ship with basic search |
| **R9**  | Scope creep from stakeholder feedback      | High        | Medium | Strict change control; defer to P3        | Lock scope at sprint boundaries                    |
| **R10** | Key developer unavailable                  | Low         | High   | Cross-training; documented specs          | Redistribute tasks; extend sprint if needed        |

---

## 4. Acceptance Criteria Summary

### 4.1 Performance Requirements

| Metric                         | Current | Target  | Measurement     |
| ------------------------------ | ------- | ------- | --------------- |
| Time to First Contentful Paint | ~300ms  | < 200ms | Lighthouse      |
| Time to Interactive            | ~800ms  | < 500ms | Lighthouse      |
| Streaming token latency        | ~100ms  | < 50ms  | Custom timing   |
| Conversation list load         | ~120ms  | < 80ms  | Network panel   |
| Search response time           | N/A     | < 100ms | Server logs     |
| Scroll FPS during streaming    | 45-50   | 60      | Chrome DevTools |

### 4.2 Quality Requirements

| Metric                      | Target  | Measurement             |
| --------------------------- | ------- | ----------------------- |
| E2E test coverage           | > 85%   | Istanbul                |
| Unit test coverage          | > 70%   | Vitest                  |
| Accessibility score         | 100     | Lighthouse              |
| TypeScript strict mode      | Enabled | tsconfig                |
| Zero console errors         | Yes     | E2E runner              |
| Zero markdown render errors | Yes     | Error boundary tracking |

### 4.3 User Experience Requirements

| Requirement                              | Validation Method                    |
| ---------------------------------------- | ------------------------------------ |
| Tables render correctly during streaming | Manual test with various table sizes |
| Code blocks preserve syntax highlighting | Screenshot comparison                |
| Math formulas render without errors      | KaTeX error count = 0                |
| Mobile history panel accessible          | Touch gesture testing                |
| Keyboard navigation complete             | Tab through all interactive elements |
| Screen reader friendly                   | VoiceOver/NVDA testing               |

---

## 5. Definition of Done

### For Each Task

- [ ] Code reviewed and approved
- [ ] Unit tests written and passing
- [ ] Integration tests (if applicable) passing
- [ ] No TypeScript errors
- [ ] No ESLint warnings
- [ ] Storybook story created (for UI components)
- [ ] Documentation updated
- [ ] Deployed to staging and verified
- [ ] Product owner sign-off

### For Each Sprint

- [ ] All P0 and P1 tasks complete
- [ ] Demo to stakeholders
- [ ] Retrospective conducted
- [ ] Sprint metrics documented
- [ ] Technical debt logged
- [ ] Next sprint planned

### For Overall Project

- [ ] All acceptance criteria met
- [ ] Performance targets achieved
- [ ] Documentation complete
- [ ] Training materials (if needed)
- [ ] Deployment runbook updated
- [ ] Monitoring dashboards configured
- [ ] Feature flag cleanup planned

---

## 6. Post-Launch Monitoring

### 6.1 Key Metrics to Track

| Metric                            | Source         | Alert Threshold    |
| --------------------------------- | -------------- | ------------------ |
| Error rate (markdown render)      | Sentry         | > 0.1%             |
| P95 streaming latency             | Custom metrics | > 200ms            |
| API error rate                    | Server logs    | > 1%               |
| Conversation load time P95        | RUM            | > 500ms            |
| User engagement (queries/session) | Analytics      | -20% from baseline |

### 6.2 Feature Flag Strategy

```typescript
// Feature flags for gradual rollout
const queryPageFeatures = {
  "query.markdown.table-buffering": true, // Sprint 1, 100%
  "query.markdown.github-alerts": true, // Sprint 2, 100%
  "query.markdown.footnotes": 0.5, // Sprint 2, 50% rollout
  "query.ui.streaming-animations": true, // Sprint 2, 100%
  "query.ui.enhanced-search": 0.2, // Sprint 2, 20% rollout
  "query.api.message-pagination": true, // Sprint 1, 100%
};
```

---

## References

- [Audit Findings](./01_audit_findings.md)
- [Design Strategy](./02_design_strategy.md)
- [Technical Specification](./03_technical_spec.md)
- [Design Mockups](./05_design_mockups.md)

---

_Document Version: 1.0 | Last Updated: December 27, 2025_
