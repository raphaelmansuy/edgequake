# Phase 4: Implementation Roadmap

**Document**: `04_implementation_roadmap.md`  
**Created**: 2024-12-27  
**Status**: Complete

---

## 1. Executive Summary

This roadmap outlines a **12-week implementation plan** organized into 3 sprints of 4 weeks each. The plan prioritizes critical fixes first (P0), followed by core improvements (P1), and concludes with enhancements (P2).

**Key Milestones**:

- **Week 4**: Streaming markdown rendering fixed, no more raw text display
- **Week 8**: Server-side persistence live, localStorage migration complete
- **Week 12**: Full feature parity with competitive products

---

## 2. Prioritization Matrix

### 2.1 Priority Definitions

| Priority | Impact | Urgency      | Description                             |
| -------- | ------ | ------------ | --------------------------------------- |
| **P0**   | High   | Critical     | Broken functionality causing user churn |
| **P1**   | High   | Important    | Core experience improvements            |
| **P2**   | Medium | Nice-to-have | Competitive feature parity              |
| **P3**   | Low    | Future       | Innovation and delight                  |

### 2.2 Issue Prioritization

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PRIORITY MATRIX                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  IMPACT                                                                      │
│    ▲                                                                         │
│    │                                                                         │
│  HIGH │  P1: Server Persistence     │  P0: Streaming Markdown              │
│    │      Pagination/Filters        │      Code Block Rendering            │
│    │      Share/Export              │      KaTeX Math Support              │
│    │                                │                                       │
│    │─────────────────────────────────────────────────────────────────────── │
│    │                                │                                       │
│  MED │  P3: Keyboard Shortcuts      │  P2: Conversation Folders            │
│    │      Custom Themes             │      Batch Operations                │
│    │      AI Title Gen              │      Responsive Mobile               │
│    │                                │                                       │
│    └───────────────────────────────────────────────────────────────▶        │
│                        LOW                              HIGH                 │
│                                  URGENCY                                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Sprint Breakdown

### 3.1 Sprint 1: Foundation (Weeks 1-4)

**Theme**: Fix Critical Rendering Issues

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ SPRINT 1: FOUNDATION                                           Weeks 1-4    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│ Week 1: Streaming Markdown Architecture                                      │
│ ─────────────────────────────────────────                                    │
│ [ ] Install marked.js + marked-katex-extension                               │
│ [ ] Create StreamingMarkdownParser class                                     │
│ [ ] Implement buffer management with safe split points                       │
│ [ ] Write unit tests for streaming parser                                    │
│                                                                              │
│ Week 2: Token-Based Rendering                                                │
│ ─────────────────────────────────                                            │
│ [ ] Create TokenRenderer component                                           │
│ [ ] Implement individual token components (Paragraph, Heading, Code, etc.)   │
│ [ ] Migrate from react-markdown to marked-based approach                     │
│ [ ] Add partial text display for incomplete blocks                           │
│                                                                              │
│ Week 3: Code Block & Mermaid Improvements                                    │
│ ────────────────────────────────────────                                     │
│ [ ] Replace prism with shiki for syntax highlighting                         │
│ [ ] Add copy-to-clipboard with toast notification                            │
│ [ ] Fix Mermaid rendering during streaming (show placeholder)                │
│ [ ] Implement lazy loading for Mermaid component                             │
│                                                                              │
│ Week 4: KaTeX & Testing                                                      │
│ ─────────────────────────                                                    │
│ [ ] Enable KaTeX support via marked extension                                │
│ [ ] Add error boundaries for malformed math                                  │
│ [ ] Comprehensive E2E tests for markdown rendering                           │
│ [ ] Performance profiling and optimization                                   │
│                                                                              │
│ DELIVERABLES:                                                                │
│ ✓ Streaming markdown renders correctly (no raw text fallback)                │
│ ✓ Code blocks with syntax highlighting + copy button                         │
│ ✓ KaTeX math equations enabled                                               │
│ ✓ Mermaid diagrams render after streaming complete                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Sprint 2: Persistence (Weeks 5-8)

**Theme**: Server-Side Storage & Sync

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ SPRINT 2: PERSISTENCE                                          Weeks 5-8    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│ Week 5: Database Schema & API                                                │
│ ─────────────────────────────────                                            │
│ [ ] Create PostgreSQL migration for conversations/messages tables            │
│ [ ] Implement CRUD handlers in Axum                                          │
│ [ ] Add cursor-based pagination                                              │
│ [ ] Write API integration tests                                              │
│                                                                              │
│ Week 6: React Query Integration                                              │
│ ───────────────────────────────                                              │
│ [ ] Create API client with fetch + error handling                            │
│ [ ] Set up React Query providers                                             │
│ [ ] Implement useConversations, useConversation hooks                        │
│ [ ] Add optimistic updates for messages                                      │
│                                                                              │
│ Week 7: Conversation List Refactor                                           │
│ ────────────────────────────────                                             │
│ [ ] Replace localStorage-based ConversationHistoryPanel                      │
│ [ ] Implement virtualized list with @tanstack/react-virtual                  │
│ [ ] Add filter bar (mode, date range, search)                                │
│ [ ] Implement infinite scroll pagination                                     │
│                                                                              │
│ Week 8: Migration & Sync                                                     │
│ ──────────────────────────                                                   │
│ [ ] Build localStorage → server migration wizard                             │
│ [ ] Implement conflict resolution UI                                         │
│ [ ] Add sync status indicator                                                │
│ [ ] Clean up old localStorage code                                           │
│                                                                              │
│ DELIVERABLES:                                                                │
│ ✓ Conversations persist to PostgreSQL                                        │
│ ✓ Paginated history with filters                                             │
│ ✓ One-time migration from localStorage                                       │
│ ✓ Cross-device conversation access                                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Sprint 3: Polish (Weeks 9-12)

**Theme**: UX Refinement & Feature Parity

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ SPRINT 3: POLISH                                               Weeks 9-12   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│ Week 9: Organization Features                                                │
│ ─────────────────────────────                                                │
│ [ ] Implement folder system for conversations                                │
│ [ ] Add pin/archive functionality                                            │
│ [ ] Create batch selection + bulk operations                                 │
│ [ ] Add rename conversation inline editing                                   │
│                                                                              │
│ Week 10: Sharing & Export                                                    │
│ ────────────────────────────                                                 │
│ [ ] Generate shareable links (public read-only)                              │
│ [ ] Export to Markdown format                                                │
│ [ ] Export to JSON format                                                    │
│ [ ] Add print-friendly view                                                  │
│                                                                              │
│ Week 11: Responsive & Mobile                                                 │
│ ────────────────────────────                                                 │
│ [ ] Implement slide-over panels for mobile                                   │
│ [ ] Add swipe gestures for history panel                                     │
│ [ ] Optimize touch targets for mobile                                        │
│ [ ] Test on various device sizes                                             │
│                                                                              │
│ Week 12: Accessibility & Final Polish                                        │
│ ─────────────────────────────────────                                        │
│ [ ] ARIA labels and keyboard navigation                                      │
│ [ ] Screen reader testing                                                    │
│ [ ] Performance audit (Lighthouse)                                           │
│ [ ] Documentation and release notes                                          │
│                                                                              │
│ DELIVERABLES:                                                                │
│ ✓ Folder organization for conversations                                      │
│ ✓ Share and export functionality                                             │
│ ✓ Mobile-responsive design                                                   │
│ ✓ WCAG 2.1 AA compliance                                                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Detailed Task Breakdown

### 4.1 Sprint 1 Tasks (Weeks 1-4)

| Task                           | Estimate | Owner | Dependencies  | Acceptance Criteria                  |
| ------------------------------ | -------- | ----- | ------------- | ------------------------------------ |
| Install marked.js + extensions | 2h       | FE    | None          | Packages in package.json             |
| StreamingMarkdownParser class  | 8h       | FE    | marked.js     | Buffer splits at safe points         |
| Token type definitions         | 2h       | FE    | marked.js     | TypeScript interfaces defined        |
| TokenRenderer component        | 4h       | FE    | Types         | Switch dispatches all token types    |
| ParagraphToken                 | 2h       | FE    | TokenRenderer | Renders inline tokens                |
| HeadingToken                   | 1h       | FE    | TokenRenderer | h1-h6 with anchors                   |
| CodeToken                      | 6h       | FE    | TokenRenderer | Syntax highlighting + copy           |
| TableToken                     | 4h       | FE    | TokenRenderer | GFM tables render                    |
| ListToken                      | 4h       | FE    | TokenRenderer | Nested lists render                  |
| BlockquoteToken                | 2h       | FE    | TokenRenderer | Styled blockquotes                   |
| MermaidToken                   | 8h       | FE    | TokenRenderer | Lazy load, placeholder during stream |
| KaTeX integration              | 6h       | FE    | marked.js     | Math equations render                |
| Shiki integration              | 6h       | FE    | CodeToken     | 20+ language themes                  |
| Migrate MarkdownRenderer       | 8h       | FE    | All tokens    | Old component replaced               |
| Unit tests (parser)            | 8h       | FE    | Parser        | >90% coverage                        |
| E2E tests (rendering)          | 8h       | FE    | All           | 10+ test scenarios                   |
| **Total Sprint 1**             | **~80h** |       |               |                                      |

### 4.2 Sprint 2 Tasks (Weeks 5-8)

| Task                        | Estimate  | Owner | Dependencies | Acceptance Criteria         |
| --------------------------- | --------- | ----- | ------------ | --------------------------- |
| DB migration file           | 4h        | BE    | None         | Tables created in Postgres  |
| Conversation CRUD handlers  | 12h       | BE    | Migration    | All endpoints working       |
| Message handlers            | 8h        | BE    | Conversation | Messages persist            |
| Cursor pagination           | 4h        | BE    | Handlers     | Cursor encodes timestamp+id |
| API client (TypeScript)     | 6h        | FE    | BE API       | Typed fetch functions       |
| React Query setup           | 4h        | FE    | Client       | QueryClient configured      |
| useConversations hook       | 4h        | FE    | RQ setup     | List with filters           |
| useConversation hook        | 4h        | FE    | RQ setup     | Single with messages        |
| useSendMessage hook         | 6h        | FE    | RQ setup     | Optimistic updates          |
| VirtualizedConversationList | 8h        | FE    | Hooks        | 1000+ items smooth          |
| FilterBar component         | 6h        | FE    | Hooks        | Mode, date, search filters  |
| InfiniteScroll trigger      | 4h        | FE    | Pagination   | Load more on scroll         |
| Migration wizard UI         | 8h        | FE    | All          | Step-by-step migration      |
| Conflict resolution         | 6h        | FE    | Wizard       | Manual merge UI             |
| Sync status indicator       | 4h        | FE    | Hooks        | Saved/Syncing/Error states  |
| Remove old localStorage     | 4h        | FE    | Migration    | Dead code removed           |
| API integration tests       | 8h        | BE    | All BE       | >80% coverage               |
| E2E persistence tests       | 8h        | FE    | All          | Create/Read/Update/Delete   |
| **Total Sprint 2**          | **~108h** |       |              |                             |

### 4.3 Sprint 3 Tasks (Weeks 9-12)

| Task                  | Estimate  | Owner | Dependencies | Acceptance Criteria        |
| --------------------- | --------- | ----- | ------------ | -------------------------- |
| Folders table + API   | 6h        | BE    | S2 complete  | CRUD for folders           |
| Folder tree component | 8h        | FE    | Folders API  | Drag-drop reorder          |
| Pin/archive API       | 4h        | BE    | S2 complete  | Update endpoints           |
| Pin/archive UI        | 4h        | FE    | API          | Toggle icons work          |
| Batch selection       | 6h        | FE    | List         | Checkbox multi-select      |
| Bulk operations       | 6h        | FE/BE | Selection    | Delete/Move/Archive        |
| Inline rename         | 4h        | FE    | List         | Double-click to edit       |
| Share link API        | 6h        | BE    | S2 complete  | Generate/revoke share IDs  |
| Share dialog UI       | 4h        | FE    | Share API    | Copy link button           |
| Export to Markdown    | 4h        | FE    | Conversation | Download .md file          |
| Export to JSON        | 2h        | FE    | Conversation | Download .json file        |
| Print view            | 4h        | FE    | Conversation | @media print styles        |
| Slide-over panels     | 6h        | FE    | None         | Mobile drawer component    |
| Swipe gestures        | 4h        | FE    | Panels       | Left/right to open/close   |
| Touch targets         | 4h        | FE    | All          | Min 44px tap areas         |
| Device testing        | 8h        | QA    | All          | iOS Safari, Android Chrome |
| ARIA labels           | 6h        | FE    | All          | All interactive elements   |
| Keyboard nav          | 6h        | FE    | All          | Full keyboard access       |
| Screen reader testing | 8h        | QA    | ARIA         | VoiceOver/NVDA tested      |
| Lighthouse audit      | 4h        | FE    | All          | >90 all categories         |
| Documentation         | 8h        | All   | All          | User guide + API docs      |
| **Total Sprint 3**    | **~112h** |       |              |                            |

---

## 5. Risk Register

| Risk                                | Likelihood | Impact   | Mitigation                                    | Contingency                     |
| ----------------------------------- | ---------- | -------- | --------------------------------------------- | ------------------------------- |
| marked.js streaming edge cases      | Medium     | High     | Extensive unit tests, fuzz testing            | Keep react-markdown as fallback |
| PostgreSQL migration data loss      | Low        | Critical | Backup before migration, transaction rollback | Restore from backup             |
| React Query cache invalidation bugs | Medium     | Medium   | Clear cache invalidation strategy             | Manual cache management         |
| Mermaid lazy load failures          | Low        | Medium   | Retry mechanism, graceful fallback            | Static placeholder              |
| Mobile gesture conflicts            | Medium     | Low      | Test early on real devices                    | Disable gestures, use buttons   |
| Bundle size increase (shiki)        | High       | Medium   | Use shiki/compat, lazy load                   | Keep prism as fallback          |
| Cross-device sync conflicts         | Medium     | High     | Clear conflict resolution UX                  | Last-write-wins with history    |

---

## 6. Gantt Chart

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              12-WEEK GANTT CHART                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ TASK                          │ W1  W2  W3  W4 │ W5  W6  W7  W8 │ W9 W10 W11 W12 │
│ ─────────────────────────────────────────────────────────────────────────── │
│                                                                             │
│ SPRINT 1: FOUNDATION                                                        │
│ ├─ Streaming Parser           │ ████            │               │               │
│ ├─ Token Components           │     ████████    │               │               │
│ ├─ Code/Mermaid/KaTeX         │         ████████│               │               │
│ └─ Testing & Optimization     │             ████│               │               │
│                                                                             │
│ SPRINT 2: PERSISTENCE                                                       │
│ ├─ DB Schema & API            │               │ ████            │               │
│ ├─ React Query Integration    │               │     ████        │               │
│ ├─ History Panel Refactor     │               │         ████    │               │
│ └─ Migration & Sync           │               │             ████│               │
│                                                                             │
│ SPRINT 3: POLISH                                                            │
│ ├─ Folders & Organization     │               │               │ ████           │
│ ├─ Sharing & Export           │               │               │     ████       │
│ ├─ Mobile & Responsive        │               │               │         ████   │
│ └─ A11y & Final Polish        │               │               │             ████│
│                                                                             │
│ ────────────────────────────────────────────────────────────────────────── │
│ MILESTONES:                                                                 │
│ ⬥ W4: Streaming markdown live                                               │
│ ⬥ W8: Server persistence live                                               │
│ ⬥ W12: Feature parity complete                                              │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Resource Requirements

### 7.1 Team Allocation

| Role               | Sprint 1 | Sprint 2 | Sprint 3 | Notes                |
| ------------------ | -------- | -------- | -------- | -------------------- |
| Frontend Developer | 100%     | 80%      | 100%     | Lead on all UI work  |
| Backend Developer  | 10%      | 60%      | 30%      | DB, API, share links |
| QA Engineer        | 20%      | 30%      | 40%      | Testing ramp-up      |
| UX Designer        | 10%      | 10%      | 20%      | Mobile patterns      |

### 7.2 Dependencies

- **marked.js v12+**: Latest stable version for streaming support
- **@tanstack/react-query v5**: Already in use, no upgrade needed
- **@tanstack/react-virtual v3**: New dependency for virtualization
- **shiki v1.0+**: New dependency for syntax highlighting
- **PostgreSQL 15+**: Existing, ensure GIN index support

---

## 8. Success Metrics

| Metric                             | Current | Target | Measurement           |
| ---------------------------------- | ------- | ------ | --------------------- |
| Markdown rendering accuracy        | ~70%    | 99%    | Manual test suite     |
| Time to render 1000 chars          | ~500ms  | <100ms | Performance profiling |
| Conversation load time (100 items) | N/A     | <500ms | Lighthouse            |
| Bundle size (query page)           | ~450KB  | <300KB | Webpack analyzer      |
| Accessibility score                | ~65     | >90    | Lighthouse            |
| Mobile usability score             | ~60     | >95    | Lighthouse            |

---

## 9. Next Steps

1. **Phase 5**: Design mockups → [05_design_mockups.md](05_design_mockups.md)
2. **Final**: Summary README → [README.md](README.md)

---

_Last updated: 2024-12-27_
