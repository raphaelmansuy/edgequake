# Query Page UX/UI Improvement Plan

> **Project**: EdgeQuake WebUI Query Page Enhancement  
> **Status**: 📋 Planning Complete  
> **Timeline**: 6 weeks (3 sprints)  
> **Specification**: [specs/18-ux-ui-adap-openwebui.md](../specs/18-ux-ui-adap-openwebui.md)

---

## 📋 Executive Summary

This plan documents a comprehensive UX/UI improvement initiative for the EdgeQuake Query Page, benchmarked against OpenWebUI's mature implementation. The goal is to achieve feature parity in markdown rendering, improve streaming performance, and enhance the overall user experience.

### Key Improvements

| Area                    | Current State            | Target State                        |
| ----------------------- | ------------------------ | ----------------------------------- |
| **Table Streaming**     | Breaks during generation | Buffered rendering with skeleton    |
| **HTML Security**       | No sanitization          | DOMPurify integration               |
| **Markdown Extensions** | Basic support            | GitHub alerts, footnotes, citations |
| **Auto-scroll**         | Janky (~45fps)           | Smooth 60fps                        |
| **Mobile History**      | Hidden sidebar           | Sheet with gestures                 |
| **Search**              | Title only               | Full-text content search            |

### Success Metrics

- **Performance**: P95 streaming latency < 50ms (current: ~100ms)
- **Quality**: 85% E2E test coverage
- **Accessibility**: Lighthouse score 100
- **Engagement**: +20% queries per session

---

## 📚 Deliverables

| Phase | Document                                                       | Description                                                           |
| ----- | -------------------------------------------------------------- | --------------------------------------------------------------------- |
| 1     | [01_audit_findings.md](./01_audit_findings.md)                 | Current state analysis, comparison with OpenWebUI, gap identification |
| 2     | [02_design_strategy.md](./02_design_strategy.md)               | Design principles, IA, interaction patterns, color system             |
| 3     | [03_technical_spec.md](./03_technical_spec.md)                 | Database schemas, API specs, component architecture, code examples    |
| 4     | [04_implementation_roadmap.md](./04_implementation_roadmap.md) | Prioritized backlog, sprint plan, risk register                       |
| 5     | [05_design_mockups.md](./05_design_mockups.md)                 | ASCII wireframes, component specifications, states                    |

---

## 🚀 Quick Start for Implementation

### Sprint 1: Foundation (Week 1-2)

**Start with these high-impact, low-risk changes:**

1. **MD-01: Table Buffering** - [Technical Spec Section 5.3](./03_technical_spec.md#53-token-completion-detection)

   ```typescript
   // Check if table is complete before rendering
   const isTableComplete = (content: string): boolean => {
     const lines = content.split("\n");
     return lines.filter((l) => l.trim().startsWith("|")).length >= 2;
   };
   ```

2. **MD-02: DOMPurify Integration** - [Technical Spec Section 5.4](./03_technical_spec.md#54-html-sanitization)

   ```bash
   pnpm add dompurify @types/dompurify
   ```

3. **MD-03: Auto-scroll Optimization** - Use `requestAnimationFrame` throttling

### Sprint 2: Extensions (Week 3-4)

4. **MD-04: GitHub Alerts** - Port from OpenWebUI's marked extension
5. **MD-05: Footnotes** - Add marked-footnote extension
6. **UI-05: Filters** - Add mode/date filters to conversation panel

### Sprint 3: Polish (Week 5-6)

7. **UI-01/02: Animations** - Streaming text fade-in, thinking shimmer
8. **MD-06: Citations** - Hover previews for sources
9. **E2E Tests** - Full coverage of query page flows

---

## 📐 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Query Page Architecture                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────────┐    ┌──────────────────────────────────────┐   │
│  │  ConversationHistory │    │          QueryInterface               │   │
│  │  Panel (Left)        │    │                                       │   │
│  │                      │◄───┤  ┌─────────────────────────────────┐  │   │
│  │  • Search            │    │  │      ChatMessage[]              │  │   │
│  │  • Filters           │    │  │                                 │  │   │
│  │  • VirtualList       │    │  │  ┌─────────────────────────┐   │  │   │
│  │                      │    │  │  │ StreamingMarkdownRenderer│   │  │   │
│  └──────────────────────┘    │  │  │                         │   │  │   │
│                              │  │  │  • marked.js + extensions│   │  │   │
│                              │  │  │  • DOMPurify            │   │  │   │
│                              │  │  │  • Lazy code blocks     │   │  │   │
│                              │  │  └─────────────────────────┘   │  │   │
│                              │  └─────────────────────────────────┘  │   │
│                              │                                       │   │
│                              │  ┌─────────────────────────────────┐  │   │
│                              │  │       InputArea                 │  │   │
│                              │  │  • Mode selector                │  │   │
│                              │  │  • Auto-resize textarea         │  │   │
│                              │  └─────────────────────────────────┘  │   │
│                              └──────────────────────────────────────┘   │
│                                                                          │
├─────────────────────────────────────────────────────────────────────────┤
│                              State Layer                                 │
│  ┌────────────────────┐   ┌────────────────────┐   ┌─────────────────┐  │
│  │  useQueryUIStore   │   │  useConversations  │   │  React Query    │  │
│  │  (Zustand)         │   │  (React Query)     │   │  Cache          │  │
│  └────────────────────┘   └────────────────────┘   └─────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## ⚠️ Key Risks & Mitigations

| Risk                                  | Impact | Mitigation                                        |
| ------------------------------------- | ------ | ------------------------------------------------- |
| Table buffering breaks other elements | High   | Comprehensive test suite                          |
| DOMPurify adds bundle size            | Medium | Dynamic import on first HTML token                |
| Search performance degrades           | High   | PostgreSQL full-text indexes, monitor query times |
| Mobile sheet conflicts with iOS       | Medium | Test on real devices, Safari-specific fixes       |

See [Risk Register](./04_implementation_roadmap.md#3-risk-register) for complete list.

---

## 🔧 Development Setup

```bash
# Clone and install
cd edgequake_webui
pnpm install

# Run development server
pnpm dev

# Run tests
pnpm test

# Run E2E tests
pnpm exec playwright test
```

### Feature Flags

```typescript
// src/lib/feature-flags.ts
export const QUERY_PAGE_FEATURES = {
  "query.markdown.table-buffering": true,
  "query.markdown.github-alerts": true,
  "query.markdown.footnotes": true,
  "query.ui.streaming-animations": true,
  "query.api.message-pagination": true,
} as const;
```

---

## 📊 Tracking Progress

### Sprint 1 Checklist

- [ ] MD-01: Table buffering implemented
- [ ] MD-02: DOMPurify integrated
- [ ] MD-03: Auto-scroll optimized
- [ ] UI-04: Mobile history sheet
- [ ] API-01: Message pagination
- [ ] DB-01: Message count materialized

### Sprint 2 Checklist

- [ ] MD-04: GitHub alerts extension
- [ ] MD-05: Footnotes extension
- [ ] UI-01: Streaming animations
- [ ] UI-02: Thinking animations
- [ ] API-02: Enhanced search
- [ ] UI-05: Filter UI

### Sprint 3 Checklist

- [ ] MD-06: Enhanced citations
- [ ] DB-02: Message versioning
- [ ] UI-03: Empty state polish
- [ ] E2E test suite complete
- [ ] Performance audit passed
- [ ] Documentation updated

---

## 👥 Contributors

- **Design**: UX audit, design strategy, mockups
- **Frontend**: React components, markdown extensions, animations
- **Backend**: API endpoints, database migrations, search optimization
- **QA**: E2E tests, accessibility testing, performance validation

---

## 📅 Timeline

```
Week 1-2: Sprint 1 - Foundation & Critical Fixes
  ├─ Fix table streaming (P0)
  ├─ Add DOMPurify (P0)
  ├─ Optimize auto-scroll (P1)
  └─ Mobile history sheet (P1)

Week 3-4: Sprint 2 - Markdown Extensions & Polish
  ├─ GitHub alerts (P1)
  ├─ Footnotes (P2)
  ├─ Streaming animations (P2)
  └─ Search & filters (P1)

Week 5-6: Sprint 3 - Advanced Features & Quality
  ├─ Enhanced citations (P2)
  ├─ Message versioning (P2)
  ├─ E2E test coverage (P1)
  └─ Performance audit (P1)
```

---

## 📚 References

- [OpenWebUI Repository](https://github.com/open-webui/open-webui) - Benchmark implementation
- [Marked.js Documentation](https://marked.js.org/) - Markdown parser
- [DOMPurify](https://github.com/cure53/DOMPurify) - HTML sanitization
- [EdgeQuake Architecture](../docs/0002-architecture-overview.md) - System context

---

_Last Updated: December 27, 2025_
