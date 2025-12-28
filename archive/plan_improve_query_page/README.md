# Query Page UX/UI Improvement Plan

**Project**: EdgeQuake WebUI Query Page Redesign  
**Created**: 2024-12-27  
**Author**: UX/UI Audit & Design Team  
**Status**: 📋 Planning Complete

---

## Executive Summary

This comprehensive improvement plan addresses critical UX issues in the EdgeQuake Query Page, focusing on four key problem areas:

1. **Markdown Rendering Failures** - Streaming content falls back to raw text
2. **State Persistence** - localStorage-only storage (5MB limit, no sync)
3. **Performance** - No pagination for conversation history
4. **Design Debt** - Inconsistent patterns and accessibility gaps

### Key Outcomes

| Metric                 | Current | Target | Improvement |
| ---------------------- | ------- | ------ | ----------- |
| Markdown accuracy      | ~70%    | 99%    | +29%        |
| Conversation load time | N/A     | <500ms | Measurable  |
| Bundle size            | ~450KB  | <300KB | -33%        |
| Accessibility score    | ~65     | >90    | +25         |

### Implementation Timeline

- **Sprint 1 (Weeks 1-4)**: Fix streaming markdown, code blocks, KaTeX
- **Sprint 2 (Weeks 5-8)**: Server persistence, pagination, migration
- **Sprint 3 (Weeks 9-12)**: Organization, sharing, mobile, accessibility

---

## Document Index

### Planning & Strategy Documents

| Phase | Document                                                     | Description                                                      |
| ----- | ------------------------------------------------------------ | ---------------------------------------------------------------- |
| 1     | [01_audit_findings.md](01_audit_findings.md)                 | User journey map, competitive analysis, technical debt inventory |
| 2     | [02_design_strategy.md](02_design_strategy.md)               | Design principles (SLICK), IA, interaction patterns              |
| 3     | [03_technical_spec.md](03_technical_spec.md)                 | Database schema, API spec, rendering pipeline                    |
| 4     | [04_implementation_roadmap.md](04_implementation_roadmap.md) | 12-week sprint plan, task breakdown, risk register               |
| 5     | [05_design_mockups.md](05_design_mockups.md)                 | ASCII wireframes, component specs, design tokens                 |

### Backend Implementation Documents

| Doc | Document                                                   | Description                                |
| --- | ---------------------------------------------------------- | ------------------------------------------ |
| 6   | [06_server_implementation.md](06_server_implementation.md) | Rust backend: handlers, services, database |

### Client-Side Implementation Documents (NEW)

| Doc | Document                                                         | Description                                   | Lines |
| --- | ---------------------------------------------------------------- | --------------------------------------------- | ----- |
| 7   | [07_client_markdown_pipeline.md](07_client_markdown_pipeline.md) | Token-based markdown rendering with marked.js | ~350  |
| 8   | [08_client_api_client.md](08_client_api_client.md)               | TypeScript API client and React Query hooks   | ~400  |
| 9   | [09_client_state_management.md](09_client_state_management.md)   | Zustand + React Query state architecture      | ~380  |
| 10  | [10_client_history_panel.md](10_client_history_panel.md)         | Virtualized history panel with filtering      | ~400  |

### Working Documents

| Doc | Document                       | Description                                  |
| --- | ------------------------------ | -------------------------------------------- |
| -   | [scratchpad.md](scratchpad.md) | Research notes and competitive analysis data |
| -   | [plan.md](plan.md)             | Action log and decision tracking             |

---

## Quick Reference

### Priority Issues (P0 - Critical)

| Issue                        | Current State             | Solution                             |
| ---------------------------- | ------------------------- | ------------------------------------ |
| Streaming markdown raw       | Falls back to plain text  | Token-based rendering with marked.js |
| Code blocks break mid-stream | Syntax highlighting fails | Buffer until code block closes       |
| KaTeX disabled               | `enableMath = false`      | marked-katex-extension integration   |
| Mermaid fails during stream  | Empty placeholder         | Render after streaming complete      |

### Database Schema (New Tables)

```sql
-- Core tables for server-side persistence
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    workspace_id UUID,
    user_id VARCHAR(255) NOT NULL,
    title VARCHAR(500),
    mode VARCHAR(50) DEFAULT 'hybrid',
    is_pinned BOOLEAN DEFAULT FALSE,
    is_archived BOOLEAN DEFAULT FALSE,
    folder_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    tokens_used INTEGER,
    context JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Key API Endpoints

| Method | Endpoint                              | Description                   |
| ------ | ------------------------------------- | ----------------------------- |
| GET    | `/api/v1/conversations`               | List with cursor pagination   |
| POST   | `/api/v1/conversations`               | Create new conversation       |
| GET    | `/api/v1/conversations/{id}`          | Get with messages             |
| POST   | `/api/v1/conversations/{id}/messages` | Add message + stream response |
| POST   | `/api/v1/conversations/import`        | Migrate from localStorage     |

### Component Architecture

```
src/components/query/
├── QueryPage.tsx                 # Page component
├── QueryInterface.tsx            # Main orchestrator
├── ConversationHistoryPanel/     # Left rail
│   ├── ConversationList.tsx      # Virtualized list
│   └── FilterBar.tsx             # Search + filters
├── Message/                      # Message rendering
│   ├── UserMessage.tsx
│   └── AssistantMessage.tsx
└── markdown/                     # Token-based renderer
    ├── TokenRenderer.tsx
    └── tokens/
        ├── CodeToken.tsx
        ├── TableToken.tsx
        └── MermaidToken.tsx
```

---

## Competitive Analysis Summary

### EdgeQuake vs openwebui

| Feature              | EdgeQuake (Current)   | openwebui            | Gap      |
| -------------------- | --------------------- | -------------------- | -------- |
| Streaming markdown   | ❌ Falls back to text | ✅ Token-based       | Critical |
| Server persistence   | ❌ localStorage only  | ✅ SQLite/PostgreSQL | Critical |
| Conversation folders | ❌ None               | ✅ Hierarchical      | P2       |
| Share links          | ❌ None               | ✅ Public links      | P2       |
| Export               | ❌ None               | ✅ MD/JSON           | P2       |

### Key Insight from openwebui

openwebui uses **token-based rendering** with `marked.lexer()` to tokenize markdown first, then renders tokens progressively. This prevents the "raw text fallback" issue EdgeQuake currently has.

```typescript
// openwebui pattern (adapted for React)
import { marked } from "marked";

function renderStreaming(content: string, done: boolean) {
  const tokens = marked.lexer(content);
  return <TokenRenderer tokens={tokens} isStreaming={!done} />;
}
```

---

## Getting Started

### For Backend/Rust Developers

1. **Start with Phase 6** ([06_server_implementation.md](06_server_implementation.md)) for the complete Rust backend guide
2. **Reference Phase 3** ([03_technical_spec.md](03_technical_spec.md)) for API contract definitions
3. **Check Phase 4** ([04_implementation_roadmap.md](04_implementation_roadmap.md)) for Sprint 2 backend tasks

### For Frontend Developers

1. **Read Phase 3** ([03_technical_spec.md](03_technical_spec.md)) for implementation details
2. **Check Phase 4** ([04_implementation_roadmap.md](04_implementation_roadmap.md)) for sprint tasks
3. **Reference Phase 5** ([05_design_mockups.md](05_design_mockups.md)) for component specs

### For Designers

1. **Review Phase 2** ([02_design_strategy.md](02_design_strategy.md)) for design principles
2. **Check Phase 5** ([05_design_mockups.md](05_design_mockups.md)) for wireframes and tokens

### For Product

1. **Start with Phase 1** ([01_audit_findings.md](01_audit_findings.md)) for problem context
2. **Review Phase 4** ([04_implementation_roadmap.md](04_implementation_roadmap.md)) for timeline

---

## Success Criteria

### Sprint 1 (Week 4)

- [ ] Streaming markdown renders correctly without raw text fallback
- [ ] Code blocks have syntax highlighting + copy button
- [ ] KaTeX math equations render properly
- [ ] Mermaid diagrams show placeholder during streaming

### Sprint 2 (Week 8)

- [ ] Conversations persist to PostgreSQL
- [ ] Paginated history with filters
- [ ] One-time migration from localStorage complete
- [ ] Cross-device conversation sync works

### Sprint 3 (Week 12)

- [ ] Folder organization implemented
- [ ] Share links functional
- [ ] Mobile-responsive design complete
- [ ] WCAG 2.1 AA compliance achieved

---

## Risk Register (Top 3)

| Risk                             | Mitigation                                            |
| -------------------------------- | ----------------------------------------------------- |
| marked.js streaming edge cases   | Extensive unit tests, keep react-markdown as fallback |
| localStorage migration data loss | Transaction rollback, manual backup option            |
| Bundle size increase from shiki  | Lazy load, use shiki/compat subset                    |

---

## Dependencies

### New Packages (Sprint 1)

- `marked` v12+ - Markdown lexer/parser
- `marked-katex-extension` - KaTeX integration
- `shiki` v1.0+ - Syntax highlighting (replaces prism)

### New Packages (Sprint 2)

- `@tanstack/react-virtual` v3 - Virtualized lists

### Existing (No Changes)

- `@tanstack/react-query` v5 - Server state
- `zustand` - Client state
- `mermaid` - Diagrams (already lazy loaded)

---

## Contact

For questions about this plan, reach out to:

- **UX/Design**: Design team Slack channel
- **Frontend**: Frontend team Slack channel
- **Backend**: Backend team Slack channel

---

_Document generated: 2024-12-27_
