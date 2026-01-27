# UX/UI Audit & Improvement Plan Prompt - Query Page (EdgeQuake WebUI)

## Role & Objective

You are a a full stack developper expert in Rust and   NextJS 16 specializing in modern, minimalist interfaces. Your mission is to implement the backend server-side components for the **Query Page** of EdgeQuake WebUI, transforming it into a best-in-class experience that exemplifies **SLICKness** and **MINIMALISM** while solving critical technical debt.
---


Fully implement plan_improve_query_page/ plan. Read carefully the research scratchpad and the UX/UI mapping plan to understand the context and the requirements.

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

