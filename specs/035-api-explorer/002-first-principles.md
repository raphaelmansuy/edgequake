# SPEC-035 — First Principles Analysis

**Lens:** First Principles Reasoning  
**Method:** Decompose the problem to its axiomatic fundamentals; rebuild the solution from scratch  

---

## Step 1 — What Are We Actually Trying to Solve?

Before evaluating options, strip all assumptions and ask: **what is the core job to be done?**

> **A developer or business user wants to understand and interact with the EdgeQuake API without writing a single line of code — directly from the application's UI.**

That's it. Everything else is implementation detail.

---

## Step 2 — What Axioms Are Absolutely True?

These are ground truths derived from the codebase:

| #   | Axiom                                                                                               | Evidence                                           |
| --- | --------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| A1  | The API contract lives in Rust source code via `#[utoipa::path]` annotations                        | `edgequake/crates/edgequake-api/src/handlers/*.rs` |
| A2  | The OpenAPI 3.0 spec is auto-generated from A1 at build time and served at `/api-docs/openapi.json` | `server.rs:L127–L135`                              |
| A3  | The spec currently documents 169 endpoints                                                          | Counted from `openapi.rs` paths() block            |
| A4  | The custom explorer hardcodes 30 endpoints from memory                                              | `api-explorer.tsx:L48–L95`                         |
| A5  | A4 cannot stay in sync with A1 without permanent manual effort                                      | Structural — no automated link exists              |
| A6  | The backend already serves a full Swagger UI at `/swagger-ui/`                                      | `server.rs:L130`, `utoipa_swagger_ui` crate        |
| A7  | The frontend is a React 19 / Next.js app                                                            | `package.json`                                     |
| A8  | The frontend uses a dark-mode-first design system (Tailwind + Radix UI)                             | `globals.css`, `design-tokens.css`                 |
| A9  | Authenticated endpoints require `Authorization: Bearer <token>`                                     | Verified in `middleware.rs`                        |
| A10 | The auth token is stored in Zustand after login                                                     | `stores/` directory                                |

---

## Step 3 — What Properties Must the Solution Have?

Derived from the axioms, not from historical choices:

| Property                              | Reason                                                   | Priority |
| ------------------------------------- | -------------------------------------------------------- | -------- |
| **P1: Zero drift**                    | A4↔A1 divergence is structurally inevitable without this | MUST     |
| **P2: Complete endpoint coverage**    | All 169 endpoints must be accessible                     | MUST     |
| **P3: Path parameter inputs**         | Without this, parameterized endpoints are untestable     | MUST     |
| **P4: Auth token injection**          | Without this, 95%+ of endpoints return 401               | MUST     |
| **P5: Request body schema**           | User must see what fields to send                        | MUST     |
| **P6: Response schema**               | User must understand what they'll receive                | MUST     |
| **P7: Visual consistency**            | App-level dark mode, design system alignment             | SHOULD   |
| **P8: Workspace context**             | Explorer should know current workspace/tenant            | SHOULD   |
| **P9: Zero maintenance per endpoint** | O(0) cost when adding new API endpoints                  | SHOULD   |
| **P10: Try-it-out**                   | Real HTTP calls from the UI                              | MUST     |

---

## Step 4 — What Solutions Exist?

### Option A: Redirect to `/swagger-ui/`
Pure forward — clicking "API Explorer" navigates to `http://localhost:8080/swagger-ui/`.

| Property              | Met? | Notes                                     |
| --------------------- | ---- | ----------------------------------------- |
| P1 Zero drift         | ✅    | Served by backend, always in sync         |
| P2 Complete coverage  | ✅    | All 169 endpoints                         |
| P3 Path params        | ✅    | Full Swagger UI support                   |
| P4 Auth injection     | ⚠️    | Manual: user types token in Swagger UI    |
| P5 Body schema        | ✅    | Full schema from OpenAPI                  |
| P6 Response schema    | ✅    | Full schema from OpenAPI                  |
| P7 Visual consistency | ❌    | Completely different design, separate URL |
| P8 Workspace context  | ❌    | Requires manual base URL entry            |
| P9 Zero maintenance   | ✅    | Fully automatic                           |
| P10 Try-it-out        | ✅    | Native Swagger UI feature                 |

**Score: 7/10** | **Effort: Near-zero** | **UX: Poor** (jarring design break, no auth/workspace integration)

---

### Option B: Embed OpenAPI React component (`@scalar/api-reference` or `swagger-ui-react`)

A React component that fetches `/api-docs/openapi.json` and renders the full interactive explorer, themed to match the EdgeQuake design system.

| Property              | Met? | Notes                                              |
| --------------------- | ---- | -------------------------------------------------- |
| P1 Zero drift         | ✅    | Reads from live spec URL                           |
| P2 Complete coverage  | ✅    | All 169 endpoints, no manual list                  |
| P3 Path params        | ✅    | Full OpenAPI param handling                        |
| P4 Auth injection     | ✅    | Token injected programmatically from Zustand store |
| P5 Body schema        | ✅    | Full schema from OpenAPI                           |
| P6 Response schema    | ✅    | Full schema from OpenAPI                           |
| P7 Visual consistency | ✅    | CSS variable theming matches design system         |
| P8 Workspace context  | ✅    | Base URL injected from workspace context           |
| P9 Zero maintenance   | ✅    | New endpoints appear automatically                 |
| P10 Try-it-out        | ✅    | Core library feature                               |

**Score: 10/10** | **Effort: ~2 days** | **UX: Excellent**

---

### Option C: Enhance custom explorer to consume OpenAPI spec dynamically

Keep the custom component but rewrite it to fetch `/api-docs/openapi.json` and generate the UI.

| Property              | Met?              | Notes                               |
| --------------------- | ----------------- | ----------------------------------- |
| P1 Zero drift         | ✅ (if done right) | Reads spec dynamically              |
| P2 Complete coverage  | ✅ (if done right) | Reads all paths from spec           |
| P3 Path params        | ⚠️                 | Must implement from scratch         |
| P4 Auth injection     | ⚠️                 | Must implement from scratch         |
| P5 Body schema        | ⚠️                 | Must implement JSON Schema renderer |
| P6 Response schema    | ⚠️                 | Must implement JSON Schema renderer |
| P7 Visual consistency | ✅                 | Full control over design            |
| P8 Workspace context  | ⚠️                 | Must implement from scratch         |
| P9 Zero maintenance   | ✅                 | Dynamic consumption                 |
| P10 Try-it-out        | ⚠️                 | Already exists for basic cases      |

**Score: 7/10** | **Effort: 3–5 weeks** | **UX: Depends on implementation quality**

> **This is essentially re-implementing a Swagger UI renderer.** It violates the "Don't build what you can use" principle and creates a new long-term maintenance burden.

---

### Option D: Keep the custom explorer, expand the hardcoded list

Current state + manual addition of missing endpoints.

| Property              | Met? | Notes                                 |
| --------------------- | ---- | ------------------------------------- |
| P1 Zero drift         | ❌    | Will drift immediately on next commit |
| P2 Complete coverage  | ⚠️    | Only if constantly maintained         |
| P3 Path params        | ❌    | Not implemented                       |
| P4 Auth injection     | ❌    | Not implemented                       |
| P5 Body schema        | ❌    | Static example JSON only              |
| P6 Response schema    | ❌    | Not shown                             |
| P7 Visual consistency | ✅    | Already in design system              |
| P8 Workspace context  | ❌    | Not implemented                       |
| P9 Zero maintenance   | ❌    | O(n) per endpoint                     |
| P10 Try-it-out        | ✅    | Basic implementation                  |

**Score: 3/10** | **Effort: 1 week initial + ongoing** | **UX: Poor**

---

## Step 5 — The First Principles Verdict

From axioms A1–A10, the living OpenAPI spec at `/api-docs/openapi.json` is the **single source of truth** for the API contract. Any UI that duplicates this information becomes a second source of truth and will drift.

The correct solution is to **consume the source of truth, not duplicate it.**

```
Source of Truth:  /api-docs/openapi.json  (auto-generated from Rust code)
                           │
                           ▼
Consumer:        React OpenAPI UI component
                 (@scalar/api-reference)
                           │
                           ├── Auth token injected from Zustand
                           ├── Base URL from workspace context
                           └── Theme from CSS variables
```

**Option B is the correct first-principles answer.**  
It satisfies all 10 required properties with minimal effort and zero ongoing maintenance.

---

## Step 6 — Library Selection: `@scalar/api-reference` vs Alternatives

| Library                 | Bundle Size | Theming       | Auth Injection | React Support | Maintenance     |
| ----------------------- | ----------- | ------------- | -------------- | ------------- | --------------- |
| `@scalar/api-reference` | ~200KB gz   | Full CSS vars | ✅              | ✅ (React pkg) | Active (Scalar) |
| `swagger-ui-react`      | ~350KB gz   | Limited CSS   | ✅              | ✅             | Maintained      |
| `@stoplight/elements`   | ~400KB gz   | Good          | ✅              | ✅             | Maintained      |
| Custom (Option C)       | ~20KB gz    | Full          | ✅              | ✅             | **Our team**    |

**Winner: `@scalar/api-reference`**
- Best developer UX out of box
- Best dark mode support
- Smallest footprint
- Active development, modern API
- `@scalar/api-reference` React package wraps the core library cleanly
- Supports `proxyUrl` for CORS (useful if needed)
- Supports `authentication` prop to pre-populate Bearer token

**Alternative if Scalar doesn't meet needs: `swagger-ui-react`** (battle-tested, well-documented)
