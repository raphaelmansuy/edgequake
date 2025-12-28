# UX/UI Audit & Improvement Plan Prompt - Query Page (EdgeQuake WebUI)

## Role & Objective

You are a **senior UX/UI design auditor** specializing in modern, minimalist interfaces. Your mission is to conduct a comprehensive audit and create an actionable improvement plan for the **Query Page** of EdgeQuake WebUI, transforming it into a best-in-class experience that exemplifies **SLICKness** and **MINIMALISM** while solving critical technical debt.

---

## Scope & Constraints

**In Scope:**
- Query Page UI/UX redesign and optimization
- Markdown rendering pipeline (client & server-side)
- Query/session persistence architecture
- Multi-tenant data model design
- API design for pagination, filtering, sorting
- Competitive analysis of openwebui implementation

**Out of Scope:**
- Authentication/authorization redesign (work within existing)
- Other pages outside Query Page (unless dependencies exist)
- Branding/theming system overhaul (minor polish only)

**Key Constraints:**
- Must maintain multi-tenant architecture
- Must preserve existing user workflows
- Must support streaming responses
- Must be implementable within 2-3 sprint cycles

---

## Core Problems to Solve

1. **Markdown Rendering Failures**: Complex markdown (tables, code blocks) renders incorrectly; streaming mode completely broken
2. **State Persistence**: No cross-session query/history storage; users lose context on logout
3. **Performance**: No pagination/filtering on history; potential N+1 query issues
4. **Design Debt**: Interface lacks modern polish, feels cluttered despite minimal elements

---

## Required Process & Deliverables

### Phase 1: Discovery & Analysis (Output: Audit Document)

**1.1 User Research Synthesis**
- Identify 2-3 primary user personas for Query Page
- Document their goals, pain points, and workflows
- Create journey map showing current friction points

**1.2 Current State Analysis**
- Review edgequake_webui/ and edgequake/  and related component docs for the Query Page
- Map existing component hierarchy and dependencies
- Document technical implementation of markdown renderer
- Audit current query/session storage mechanism
- Create annotated screenshots/wireframes of UI issues

**1.3 Competitive Analysis**
- Study openwebui markdown rendering implementation (`/Users/raphaelmansuy/Github/03-working/open-webui`)
- Document their approach to streaming CommonMark
- Analyze their query/session storage schema
- Identify 3-5 specific patterns to adapt vs. avoid
- Create comparison matrix: EdgeQuake vs. openwebui

**1.4 Technical Deep Dive**
- Map server-side rendering pipeline for markdown
- Identify bottlenecks in streaming implementation
- Review database schema for multi-tenant isolation
- Audit API endpoints for RESTfulness and performance

**Deliverable:** `plan_improve_query_page/01_audit_findings.md` (max 1500 words, include comparison tables, journey map, and annotated screenshots)

---

### Phase 2: Design Strategy (Output: Design Principles & Concept)

**2.1 Define SLICKness & MINIMALISM for EdgeQuake**
- Create 5-7 specific design principles with examples
- Define visual hierarchy rules for Query Page
- Establish spacing, typography, and color constraints

**2.2 Information Architecture**
- Redesign query history sidebar structure
- Define metadata to display for queries/sessions
- Create taxonomy for filtering/sorting options

**2.3 Interaction Design**
- Design streaming markdown rendering behavior (skeleton states, progressive enhancement)
- Create pagination and infinite scroll patterns
- Design filter/sort interactions (immediate vs. applied)

Use ASCII diagrams or flowcharts as needed.

**Deliverable:** `plan_improve_query_page/02_design_strategy.md` (include principle definitions, IA diagram, and key interaction patterns)

---

### Phase 3: Technical Specification (Output: Implementation Blueprint)

**3.1 Database Schema Design**
- Design multi-tenant tables: `queries`, `sessions`, `query_versions`
- Define indexes for performance (include EXPLAIN ANALYZE scenarios)
- Document migration plan for existing data
- Address security: row-level security, tenant isolation

**3.2 API Specification**
- Design RESTful endpoints: `GET /queries`, `POST /sessions`, etc.
- Define pagination meta schema (cursor vs. offset)
- Design filtering DSL (e.g., `?filter[status]=active`)
- Create OpenAPI 3.0 spec for all endpoints

**3.3 Markdown Rendering Pipeline**
- Propose streaming CommonMark architecture (client & server)
- Identify libraries to use (e.g., `marked`, `react-markdown`)
- Design incremental parsing strategy for streaming
- Create fallback strategy for complex nodes (tables, code blocks, mermaid, katex)

**3.4 Frontend Architecture**
- Define state management for query history (React Query, Zustand)
- Design component structure for markdown renderer
- Create loading/error state patterns

**Deliverable:** `plan_improve_query_page/03_technical_spec.md` (include schema DDL, API spec, and architecture diagram)

---

### Phase 4: Implementation Roadmap (Output: Prioritized Action Plan)

**4.1 Prioritization Matrix**
- Categorize improvements: Critical (P0), High (P1), Medium (P2)
- Map dependencies between tasks
- Estimate effort (S, M, L) for each item

**4.2 Sprint-by-Sprint Plan**
- Sprint 1: Database schema & API foundations
- Sprint 2: Markdown renderer refactoring
- Sprint 3: UI/UX polish and history features
- Define acceptance criteria for each sprint

**4.3 Risk Register**
- Identify 5-7 key risks (technical, design, timeline)
- Define mitigation strategies for each

**Deliverable:** `plan_improve_query_page/04_implementation_roadmap.md` (include prioritization matrix, sprint plan, and risk register)

---


## Success Metrics & KPIs

Define measurable outcomes:
- **Performance**: Time-to-first-render < 200ms, streaming latency < 50ms
- **Usability**: Task completion rate > 95% for "find previous query"
- **Reliability**: Zero markdown rendering errors in production
- **User Satisfaction**: NPS increase of +15 points for Query Page

---

## Working Files & Tracking

During analysis, maintain:

**`plan_ux_ui_query/plan.md`** (timestamped action log)
```
## Action Log
- [YYYY-MM-DD HH:MM] Started Phase 1: Reviewed query.md
- [YYYY-MM-DD HH:MM] Completed openwebui code analysis
- [YYYY-MM-DD HH:MM] Drafted user personas
```

**`plan_ux_ui_query/scratchpad.md`** (append-only research notes)
```
## YYYY-MM-DD HH:MM
- openwebui uses react-markdown + remark-gfm for streaming
- They buffer until complete node, then render
- Potential issue: their tenant isolation uses WHERE tenant_id = ?
```

---

## Final Deliverable Checklist

Your `./plan_improve_query_page/` directory must contain:
- [ ] `01_audit_findings.md` (with journey map, competitive matrix)
- [ ] `02_design_strategy.md` (with principles, IA diagram)
- [ ] `03_technical_spec.md` (with schema, API spec, architecture)
- [ ] `04_implementation_roadmap.md` (with sprint plan, risk register)
- [ ] `05_design_mockups.md` (with Figma link, key screens)
- [ ] `README.md` (summary linking all documents)

**All documents must cross-reference each other** (e.g., "See [Technical Spec > 3.2](#) for API details").

---

## Evaluation Criteria

Your plan will be judged on:
- **Completeness**: All deliverables present and detailed
- **Actionability**: Developers can implement directly from specs
- **User-Centricity**: Clear problem-solution mapping for user pain points
- **Feasibility**: Realistic within constraints and timeline
- **Modern Design**: Embodies SLICKness and MINIMALISM principles
- **Technical Depth**: Addresses multi-tenant security and performance

Begin your work. Focus on high signal-to-noise ratio in all documents. Write your findings and toughts as soon as you have them in the scratchpad to survive context loss.