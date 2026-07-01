# SPEC-035 — Product Owner Lens

**Lens:** Product Owner / Business Value  
**Key Question:** What is the business cost of the current state, and what is the ROI of fixing it?  

---

## The Business Problem Statement

The API Explorer is positioned as a developer-facing product feature. It is listed in the main navigation sidebar. Users expect it to represent the full capability of the EdgeQuake API.

Currently, it shows **30 of 169 endpoints (17.8% coverage)**. It cannot test parameterized endpoints. It cannot authenticate requests. It actively misleads users about the API's capabilities.

This is not a minor polish issue. It is a **product credibility problem.**

---

## Business Impact — Current State

### Visible Costs

| Impact                       | Severity | Evidence                                                                            |
| ---------------------------- | -------- | ----------------------------------------------------------------------------------- |
| **Developer trust erosion**  | HIGH     | User opens explorer, finds their endpoint missing — assumes the API doesn't have it |
| **API adoption friction**    | HIGH     | Without a working explorer, developers resort to curl/Postman — onboarding friction |
| **Support burden**           | MEDIUM   | Users ask "how do I test endpoint X?" because the explorer doesn't show it          |
| **Documentation disconnect** | HIGH     | Features like workspaces, conversations, PDFs, OIDC are invisible in the explorer   |
| **Product credibility**      | HIGH     | A tool that claims to be an "API Explorer" but covers 18% is a liability            |

### Hidden Costs (Maintenance Debt)

| Cost                               | Frequency         | Time   | Total (yearly)               |
| ---------------------------------- | ----------------- | ------ | ---------------------------- |
| Add new endpoint to explorer       | Each API sprint   | 15 min | ~6 hours/year                |
| Debug "endpoint not found" issues  | Weekly            | 30 min | ~26 hours/year               |
| Keep request body examples current | Per schema change | 20 min | ~10 hours/year               |
| **Total hidden cost**              |                   |        | **~42 developer-hours/year** |

> At typical developer rates, this is **$4,000–$8,000/year in maintenance cost** for a tool that provides 18% of its promised value.

---

## Business Impact — Proposed State (Option B)

### Immediate Gains on Shipping

| Gain                              | Value                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------ |
| **100% endpoint coverage**        | All 169 endpoints accessible, including multi-tenant, PDF, conversation, graph |
| **Zero maintenance per endpoint** | New endpoints appear automatically via OpenAPI spec                            |
| **Auth token pre-populated**      | Protected endpoints testable immediately after login                           |
| **Workspace context injection**   | Correct base URL auto-configured                                               |
| **Schema documentation**          | Users see request/response shapes without reading code                         |

### ROI Calculation

|                              | Current State       | Option B         |
| ---------------------------- | ------------------- | ---------------- |
| Development effort           | 42 hrs/year ongoing | ~16 hrs one-time |
| Coverage                     | 17.8%               | 100%             |
| Maintenance per new endpoint | 15 min              | 0 min            |
| User self-service rate       | Low                 | High             |
| Break-even                   | —                   | ~3 months        |

---

## Stakeholder Impact Map

```
┌─────────────────────────────────────────────────────────────────┐
│                        STAKEHOLDER MAP                          │
├───────────────────┬────────────────────────────────────────────┤
│ Stakeholder       │ Current Pain → Solution Benefit            │
├───────────────────┼────────────────────────────────────────────┤
│ API Developer     │ Can't find/test 82% of endpoints →         │
│                   │ Full coverage, auth injection, try-it-out  │
├───────────────────┼────────────────────────────────────────────┤
│ Business Analyst  │ Can't explore API without code knowledge → │
│                   │ Visual schema docs, no-code exploration     │
├───────────────────┼────────────────────────────────────────────┤
│ Integration Eng.  │ Must use Postman/curl for testing →         │
│                   │ In-app testing with real auth tokens        │
├───────────────────┼────────────────────────────────────────────┤
│ QA Engineer       │ Manual endpoint discovery →                │
│                   │ Automated spec-driven test reference        │
├───────────────────┼────────────────────────────────────────────┤
│ Product Owner     │ Explorer is a liability →                  │
│                   │ Explorer is a differentiator               │
├───────────────────┼────────────────────────────────────────────┤
│ Support Team      │ "How do I test X?" tickets →               │
│                   │ Self-service API exploration               │
└───────────────────┴────────────────────────────────────────────┘
```

---

## Feature Parity Gap (What Users Cannot Do Today)

The following API categories are **completely absent** from the current explorer, yet are key product features:

| Missing Category  | Endpoints | Business Capability               |
| ----------------- | --------- | --------------------------------- |
| Workspaces        | 8         | Multi-tenant workspace management |
| Conversations     | 12        | Chat history management           |
| Folders           | 4         | Conversation organization         |
| PDF documents     | 11        | PDF upload, processing, status    |
| Cost tracking     | 5         | Budget/cost management            |
| User management   | 6         | User CRUD, role assignment        |
| API Keys          | 3         | Programmatic access management    |
| Injections        | 6         | RAG context injection             |
| Lineage           | 5         | Document lineage tracking         |
| Jobs              | 4         | Workspace job management          |
| OIDC/OAuth        | 4         | SSO integration                   |
| **Total missing** | **~139**  | **82% of the API**                |

---

## Product Owner Decision Criteria

### Acceptance Criteria for New API Explorer

| #    | Criterion            | Measurable Target                                              |
| ---- | -------------------- | -------------------------------------------------------------- |
| AC1  | Endpoint coverage    | 100% of endpoints in `/api-docs/openapi.json`                  |
| AC2  | Auth integration     | Bearer token pre-populated from logged-in session              |
| AC3  | Workspace awareness  | Base URL reflects current workspace                            |
| AC4  | Try-it-out works     | 200 response returned for valid GET /health call               |
| AC5  | Dark mode            | Matches EdgeQuake dark theme, no visual jarring                |
| AC6  | Path param inputs    | `/documents/{id}` renders an input for `id`                    |
| AC7  | Request body schema  | POST body shows required fields and types                      |
| AC8  | Response schema      | 200 response schema shown                                      |
| AC9  | Zero maintenance     | Adding a new endpoint to Rust code requires NO frontend change |
| AC10 | Navigation preserved | Explorer accessible from sidebar at same URL                   |

---

## Non-Goals (Out of Scope)

| Not in scope                | Reason                              |
| --------------------------- | ----------------------------------- |
| Custom endpoint grouping UI | The spec provides `tags` — use them |
| Code generation (curl/SDK)  | Nice to have, not blocking          |
| Persisting test scenarios   | Future feature                      |
| API versioning UI diff      | Future feature                      |
| Mocking/sandbox mode        | Future feature                      |

---

## Risk Assessment

| Risk                                     | Probability | Impact | Mitigation                                  |
| ---------------------------------------- | ----------- | ------ | ------------------------------------------- |
| Library bundle size impacts performance  | LOW         | MEDIUM | Lazy load the component                     |
| CORS blocks requests to `localhost:8080` | MEDIUM      | HIGH   | Configure CORS in backend (already done)    |
| Scalar update breaks compatibility       | LOW         | LOW    | Pin version, update on schedule             |
| Auth token expiry during exploration     | MEDIUM      | LOW    | Show error, prompt re-login                 |
| OpenAPI spec has incomplete annotations  | MEDIUM      | MEDIUM | Addressed in SPEC-027 (A++ enrichment done) |

---

## Sprint Sizing

| Sprint    | Deliverable                                        | Estimate      |
| --------- | -------------------------------------------------- | ------------- |
| Sprint 1  | Install `@scalar/api-reference`, basic integration | 4 hours       |
| Sprint 2  | Dark mode theming, CSS variable mapping            | 4 hours       |
| Sprint 3  | Auth token injection from Zustand                  | 3 hours       |
| Sprint 4  | Workspace base URL injection                       | 2 hours       |
| Sprint 5  | Remove old custom component, cleanup               | 2 hours       |
| Sprint 6  | E2E tests, accessibility check                     | 3 hours       |
| **Total** |                                                    | **~18 hours** |
