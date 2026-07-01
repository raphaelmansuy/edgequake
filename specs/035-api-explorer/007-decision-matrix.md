# SPEC-035 — Decision Matrix: Custom vs Swagger UI vs Hybrid

**Lens:** Decision Science  
**Method:** Multi-criteria weighted scoring + adversarial testing  

---

## The Four Options

| Option | Description                                         |
| ------ | --------------------------------------------------- |
| **A**  | Redirect / iframe to existing `/swagger-ui/`        |
| **B**  | Embed `@scalar/api-reference` React component       |
| **C**  | Rewrite custom explorer to consume spec dynamically |
| **D**  | Keep current custom explorer, maintain manually     |

---

## Weighted Scoring Matrix

### Criteria and Weights

| #   | Criterion                                  | Weight | Rationale                   |
| --- | ------------------------------------------ | ------ | --------------------------- |
| C1  | Endpoint coverage (% of API visible)       | 20%    | Core functional requirement |
| C2  | Stays in sync with API without manual work | 20%    | Prevents regression         |
| C3  | Developer UX quality                       | 15%    | Primary user is a developer |
| C4  | Visual consistency with app                | 10%    | Product quality             |
| C5  | Auth token injection from session          | 10%    | 95% of endpoints need auth  |
| C6  | Schema documentation quality               | 10%    | Developer productivity      |
| C7  | Implementation effort (less = better)      | 10%    | Engineering ROI             |
| C8  | Long-term maintenance cost (less = better) | 5%     | Sustainability              |

### Scoring (1–5, where 5 is best)

| Criterion                    | Weight | A: Redirect | B: Scalar | C: Custom rewrite | D: Status quo |
| ---------------------------- | ------ | ----------- | --------- | ----------------- | ------------- |
| C1 Coverage                  | 20%    | 5           | 5         | 5                 | 1             |
| C2 Sync                      | 20%    | 5           | 5         | 4                 | 1             |
| C3 Dev UX                    | 15%    | 3           | 5         | 3                 | 2             |
| C4 Visual consistency        | 10%    | 1           | 4         | 5                 | 5             |
| C5 Auth injection            | 10%    | 2           | 5         | 4                 | 1             |
| C6 Schema docs               | 10%    | 5           | 5         | 3                 | 1             |
| C7 Effort (less=better)      | 10%    | 5           | 4         | 2                 | 5             |
| C8 Maintenance (less=better) | 5%     | 5           | 5         | 4                 | 1             |

### Weighted Totals

| Option            | Score    | Normalized (%)   |
| ----------------- | -------- | ---------------- |
| **B: Scalar**     | **4.65** | **93%** ✅ WINNER |
| A: Redirect       | 3.80     | 76%              |
| C: Custom rewrite | 3.75     | 75%              |
| D: Status quo     | 1.90     | 38%              |

Calculation:
- **B**: 0.20×5 + 0.20×5 + 0.15×5 + 0.10×4 + 0.10×5 + 0.10×5 + 0.10×4 + 0.05×5 = **4.65**
- **A**: 0.20×5 + 0.20×5 + 0.15×3 + 0.10×1 + 0.10×2 + 0.10×5 + 0.10×5 + 0.05×5 = **3.80**

---

## Decision: **Option B — Embed `@scalar/api-reference`**

---

## Adversarial Battle-Testing of the Decision

### Attack 1: "Scalar might break or be abandoned"

**Counter:** 
- Scalar is actively maintained (GitHub: scalar/scalar, 8k+ stars, updated weekly as of 2024)
- Risk mitigation: pin the minor version; update quarterly
- Fallback: `swagger-ui-react` achieves the same outcome with different UX
- Worst case: remove Scalar, add `swagger-ui-react` — ~4 hours of work
- **The alternative (status quo) has already broken, invisibly. The known risk > unknown risk.**

### Attack 2: "We lose control over the UI/UX"

**Counter:**
- Scalar supports full CSS variable overrides
- Layout, colors, fonts — all configurable
- We control: auth injection, base URL, initial state
- We don't control: internal component layout (acceptable — it's a polished tool)
- **The custom explorer currently has broken UX (no auth, 82% blind spot). "Control" without correctness is not valuable.**

### Attack 3: "The bundle size will hurt performance"

**Counter:**
- Scalar: ~200KB gzipped (less than `swagger-ui-react` at ~350KB)
- Solution: `next/dynamic` with `ssr: false` — library only loads when user navigates to `/api-explorer`
- Impact on initial page load: zero (lazy loaded)
- **Performance concern is addressed by lazy loading. Not a valid objection.**

### Attack 4: "We should build our own for complete control"

**Counter:**
- "Complete control" over what? Building a Swagger UI replacement.
- Feature list to implement from scratch: path params, body schema, response schema, auth, search, copy-curl, syntax highlighting, JSON schema viewer, dark mode — ~3–5 weeks
- Scalar does all of this, maintained by a dedicated team
- Our team's time is better spent on EdgeQuake features, not reinventing a standard tool
- **DRY principle: do not build what you can use.**

### Attack 5: "The existing /swagger-ui/ already works — why not use it?"

**Counter:**
- Option A (redirect) is scored at 76% — not bad, but:
  1. It breaks out of the application context (separate URL, different design)
  2. Auth token NOT injected — user must manually copy/paste JWT
  3. No workspace context — user must type the base URL
  4. Visual jarring — bright white Swagger theme in a dark-mode-first application
- Option B achieves everything Option A does, plus auth injection, workspace context, and design consistency
- **Extra 4–8 hours of work for Option B vs A is worth it for the auth injection alone (eliminates manual token copy-paste for every protected endpoint).**

### Attack 6: "OpenAPI spec has bugs — embedding it will expose them"

**Counter:**
- The OpenAPI spec was extensively enriched in SPEC-027 (phases 13–15, A++ rating)
- Enrichment includes: examples, schemas, descriptions, server URLs
- Any spec bugs are bugs that would appear in Swagger UI too — they're already exposed
- Fixing spec bugs in Rust (via `utoipa` annotations) fixes them everywhere automatically
- **This is an argument FOR the OpenAPI-native approach — fixing one source fixes all consumers.**

---

## Why NOT Option A (Redirect to Swagger UI)

The backend Swagger UI is a valid tool. The specific reasons for preferring Option B:

| Problem with Option A              | Impact                                                 |
| ---------------------------------- | ------------------------------------------------------ |
| Auth token not pre-populated       | Developer must manually copy JWT from browser DevTools |
| No workspace base URL injection    | Developer must type `http://localhost:8080` manually   |
| Completely different visual design | Product feels incoherent                               |
| Separate domain context            | Breaks navigation history                              |
| No integration with app router     | Can't pass context from the app                        |

**Option A is acceptable as a fallback or complement** (e.g., a "View in Swagger UI" link), but not as the primary experience.

---

## Why NOT Option C (Custom Rewrite)

Option C would mean:
- Implementing a full JSON Schema renderer (complex tree structures)
- Implementing path parameter detection and input generation
- Implementing response schema display
- Implementing auth header injection UI
- Implementing search/filter across 169 endpoints
- Maintaining all of the above across browser versions and design system updates

This is approximately **rebuilding a subset of Swagger UI** — something that `@scalar/api-reference` already provides, maintained by a dedicated team with thousands of users.

**The only advantage of Option C** is maximum design control. But this advantage exists only if the team commits to maintaining a full OpenAPI renderer indefinitely. **This is not our core product.**

---

## Why NOT Option D (Status Quo)

Option D is not a valid choice. The current state:
- Shows 17.8% of endpoints
- Misleads users about API capabilities
- Has no auth support
- Fails silently on parameterized endpoints
- Costs ~42 developer-hours/year in maintenance
- Will continue drifting further behind as the API grows

**Keeping the status quo is choosing to have a broken feature in the product.**

---

## Final Decision Statement

```
DECISION: Replace api-explorer.tsx with @scalar/api-reference React component
          consuming /api-docs/openapi.json dynamically.

RATIONALE:
  1. DRY: eliminates 30-entry hardcoded list; one URL is the source of truth
  2. COVERAGE: 100% of 169 endpoints immediately visible
  3. UX: auth injection, schema docs, search, path params — zero custom code
  4. SYNC: new endpoints appear automatically with API development
  5. EFFORT: ~18 hours one-time vs 42+ hours/year ongoing
  6. SOLID: single-responsibility component depending on spec URL abstraction

SECONDARY RECOMMENDATION:
  Keep /swagger-ui/ accessible for power users who prefer the standalone Swagger UI.
  Add a "Open in Swagger UI" link from the explorer for alternative access.
```
