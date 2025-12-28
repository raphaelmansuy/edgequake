# Task Log: Query Page UX/UI Improvement Plan

**Date**: 2024-12-27-15-45  
**Mode**: Beastmode  
**Task**: Comprehensive UX/UI Audit and Improvement Plan for EdgeQuake Query Page

---

## Actions

- Created working directory `/plan_improve_query_page/`
- Explored EdgeQuake Query Page components: `query-interface.tsx`, `markdown-renderer.tsx`, `use-conversation-store.ts`, `conversation-history-panel.tsx`, `chat-message.tsx`
- Fetched openwebui implementation patterns from GitHub repo (marked.js tokenization, SQL schema)
- Created `scratchpad.md` with research notes
- Created `plan.md` with action tracking
- Created Phase 1: `01_audit_findings.md` (~3000 words) - user journey, competitive matrix, tech debt
- Created Phase 2: `02_design_strategy.md` (~4500 words) - SLICK principles, IA, interaction patterns
- Created Phase 3: `03_technical_spec.md` (~5000 words) - DB schema, API spec, markdown pipeline
- Created Phase 4: `04_implementation_roadmap.md` (~3500 words) - 12-week sprint plan, Gantt chart
- Created Phase 5: `05_design_mockups.md` (~4000 words) - ASCII wireframes, component specs
- Created `README.md` summary with quick reference

## Decisions

- Used `marked.js` over `react-markdown` for token-based streaming (matches openwebui pattern)
- Chose cursor-based pagination over offset (better for real-time data)
- Designed normalized schema (separate `conversations` and `messages` tables) over JSON blob
- 12-week timeline split into 3 sprints: Foundation → Persistence → Polish
- P0 priority assigned to streaming markdown fix (user-facing critical bug)

## Next Steps

1. Review documents with stakeholders
2. Create Jira/Linear tickets from Sprint 1 tasks
3. Set up development branch for markdown refactor
4. Install `marked` + `marked-katex-extension` packages
5. Create unit test suite for `StreamingMarkdownParser`

## Lessons/Insights

- openwebui's token-based approach prevents raw text fallback during streaming
- EdgeQuake's 60+ regex normalizations in markdown-renderer indicate upstream LLM tokenization issues
- localStorage persistence is architectural debt limiting cross-device UX
- marked.lexer() provides safe split points for streaming content

---

_Generated: 2024-12-27_
