# SPEC-035 — UX/UI Designer Lens

**Lens:** User Experience and Visual Design  
**Key Questions:**  
- What is the ideal explorer experience for different user types?  
- How do we achieve visual consistency without building a custom renderer?  
- What are the UX anti-patterns to avoid?  

---

## Current UX Audit — Critical Failures

### 1. Endpoint Discovery Failure

**Current UX:**
```
Category: Documents (4 items shown)
  [GET]    /documents
  [POST]   /documents
  [GET]    /documents/{id}
  [DELETE] /documents/{id}
```

**Reality (missing from explorer):**
```
Also available but hidden:
  [POST]   /documents/scan-directory
  [GET]    /documents/search
  [POST]   /documents/reprocess-failed
  [POST]   /documents/recover-stuck
  [GET]    /documents/{id}/lineage
  [GET]    /documents/{id}/metadata
  [GET]    /documents/{id}/chunk/{chunk_id}
  ... (8 more document endpoints)
```

A user exploring document processing capabilities will believe these features don't exist.

### 2. Silent Parameter Failure

```
User selects: GET /documents/{id}
User clicks: Execute
URL sent:    GET /documents/{id}   ← literal {id} string
Response:    404 Not Found
```

There is no input field for path parameters. The request fails silently. The user doesn't know if the endpoint is broken or if they need to provide an ID.

### 3. Authentication Opacity

All protected endpoints (95%+ of the API) fail with 401 Unauthorized when executed. There is no indication:
- That authentication is required
- How to provide it
- That the user is already authenticated in the app

### 4. Zero Schema Information

When a user selects `POST /query`, they see a static JSON example:
```json
{
  "query": "What is the main topic?",
  "mode": "hybrid",
  "top_k": 10
}
```

They don't know:
- Is `mode` required or optional?
- What values are valid for `mode`?
- What does the response look like?
- Are there other fields available?

---

## The Ideal UX — Three User Archetypes

### Archetype 1: The API Developer (technical integrator)

**Goal:** Build integration code against EdgeQuake API  
**Needs:**
- Complete endpoint list with HTTP method, path, description
- Full request/response schemas (for type generation)
- Authentication documentation (Bearer token, API key)
- Example requests and responses
- Ability to test endpoints with real data

**Ideal journey:**
1. Open explorer → See all endpoints grouped by resource
2. Click endpoint → See full schema, auth requirements
3. Click "Try it" → Auth already filled from session
4. Send request → See formatted response with schema overlay
5. Copy curl → Get ready-to-use command

### Archetype 2: The Business Analyst (non-technical explorer)

**Goal:** Understand what the API can do without writing code  
**Needs:**
- Plain-language descriptions of each endpoint
- Visual response examples
- No intimidating raw JSON unless they ask
- Easy navigation by feature area

**Ideal journey:**
1. Open explorer → See logical groups (Documents, Query, Graph, etc.)
2. Click category → See endpoint list with human-readable descriptions
3. Click endpoint → See what it does, what it needs, what it returns
4. Try a GET endpoint → See real response from their data

### Archetype 3: The DevOps / Integration Engineer

**Goal:** Verify endpoint availability, test health checks, configure integrations  
**Needs:**
- Health endpoints first
- Auth endpoint documentation
- Server URL configuration
- Quick way to verify the API is working

---

## UX Design Principles for the New Explorer

### Principle 1: Progressive Disclosure

Show simple information first; reveal complexity on demand.

```
Level 0 (always visible):   [GET] /documents — List all documents
Level 1 (on hover):         Parameters summary • Authentication required
Level 2 (on click):         Full parameter inputs, auth fields
Level 3 (after execute):    Request/response, schema overlay, timing
```

### Principle 2: Auth-Aware from First Interaction

When the user is logged in:
- Bearer token is pre-populated in the auth header
- Lock icon shows "Authenticated" (green)
- No manual token entry needed

When the user is NOT logged in:
- Auth field shows "Not authenticated"
- Link to login page
- Endpoints still visible for documentation browsing

### Principle 3: Workspace Context Injection

The explorer must respect the current workspace context:
```
Active workspace: Default Workspace (ID: abc-123)
Tenant: Default (ID: xyz-789)

→ Base URL: http://localhost:8080
→ Auth token: Bearer eyJ...
→ Workspace header: X-Workspace-ID: abc-123
```

### Principle 4: Visual Hierarchy that Matches EdgeQuake Design

The Scalar API reference theme should be configured to match:

| EdgeQuake Token      | Scalar CSS Variable     | Value (dark mode)        |
| -------------------- | ----------------------- | ------------------------ |
| `--background`       | `--scalar-background-1` | `hsl(222.2 84% 4.9%)`    |
| `--card`             | `--scalar-background-2` | `hsl(217.2 32.6% 17.5%)` |
| `--primary`          | `--scalar-color-accent` | `hsl(217.2 91.2% 59.8%)` |
| `--foreground`       | `--scalar-color-1`      | `hsl(210 40% 98%)`       |
| `--muted-foreground` | `--scalar-color-3`      | `hsl(215 20.2% 65.1%)`   |
| GET green            | `--scalar-color-green`  | `hsl(142 71% 45%)`       |
| POST blue            | `--scalar-color-blue`   | `hsl(217 91% 60%)`       |
| DELETE red           | `--scalar-color-red`    | `hsl(0 84% 60%)`         |
| PATCH orange         | `--scalar-color-orange` | `hsl(24.6 95% 53.1%)`    |

### Principle 5: Responsive Layout Integration

The explorer should be full-height within the dashboard layout:

```
┌─────────────────────────────────────────────────────────────┐
│  SIDEBAR  │           API EXPLORER                          │
│           │                                                 │
│ Dashboard │  ┌──────────────┬──────────────────────────┐   │
│ Documents │  │   Endpoints  │   Request / Response      │   │
│ Query     │  │   (sidebar)  │   (main area)             │   │
│ Graph     │  │              │                           │   │
│ ──────    │  │   Health  1  │   GET /health             │   │
│ API Exp ◄ │  │   Auth    2  │   ─────────────────────   │   │
│ Settings  │  │   Models  4  │   Response: 200 OK        │   │
│           │  │   Docs    15 │   { "status": "healthy" } │   │
│           │  └──────────────┴──────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## UX Anti-Patterns to Avoid

| Anti-Pattern                     | Problem                                   | Solution                                      |
| -------------------------------- | ----------------------------------------- | --------------------------------------------- |
| Blank "Select an endpoint" state | Wastes viewport, no orientation           | Show welcome state with search + quick starts |
| Silent 401 errors                | User doesn't know why it failed           | Show auth status banner; pre-populate token   |
| Literal `{id}` in requests       | Confusing failed requests                 | Path param inputs — always                    |
| Static example JSON only         | No schema understanding                   | Show full JSON Schema with descriptions       |
| No loading state                 | User doesn't know if request is in flight | Spinner with "Executing…" + timeout           |
| Truncated response               | Large responses unreadable                | Scrollable, collapsible JSON viewer           |
| No copy-as-curl                  | Developer productivity                    | Copy curl command for every request           |
| No search/filter                 | 169 endpoints, hard to navigate           | Persistent search bar at top                  |

---

## Interaction Design — Critical Flows

### Flow 1: First Time Opening Explorer

```
User lands on /api-explorer
    │
    ▼
[WELCOME STATE]
"EdgeQuake API Explorer — 169 endpoints"
[Quick searches: health, query, documents, graph]
[Auth status: ✓ Authenticated as admin@edgequake.com]
[Server: http://localhost:8080]
```

### Flow 2: Testing a Protected Endpoint

```
1. User selects POST /api/v1/query
2. Explorer shows:
   - Auth: Bearer eyJ... (pre-filled, green checkmark)
   - Request body: JSON schema with required fields highlighted
   - Example: { "query": "...", "mode": "hybrid" }
3. User types their query, clicks "Send"
4. Response shows: 200 OK + JSON with syntax highlighting
5. Response time shown: 234ms
```

### Flow 3: Discovering an Unknown Endpoint

```
1. User types "pdf" in search bar
2. Instant filter: Shows all PDF-related endpoints
   POST /api/v1/pdf-documents
   GET  /api/v1/pdf-documents
   GET  /api/v1/pdf-documents/{id}/status
   GET  /api/v1/pdf-documents/{id}/content
   POST /api/v1/pdf-documents/retry
   ... (6 more)
3. User didn't know these existed — discovery complete
```

---

## Accessibility Requirements

| Requirement          | Standard    | Notes                                       |
| -------------------- | ----------- | ------------------------------------------- |
| Keyboard navigation  | WCAG 2.1 AA | Full keyboard access to all endpoints       |
| Screen reader labels | WCAG 2.1 AA | Method badges labeled (e.g., "GET request") |
| Color contrast       | WCAG 4.5:1  | All text on dark background                 |
| Focus indicators     | WCAG 2.4.7  | Visible focus rings on interactive elements |
| Status announcements | ARIA live   | Response loaded / error occurred            |

---

## Loading and Error States

### Loading State
```
[POST /api/v1/query]
[Executing...]  ←  Spinner with 30s timeout indicator
```

### Error States

| Error                | Display                                                                       |
| -------------------- | ----------------------------------------------------------------------------- |
| 401 Unauthorized     | "Authentication required — your session may have expired" + [Re-login] button |
| 404 Not Found        | "Endpoint not found — check path parameters"                                  |
| 422 Validation Error | Highlight invalid fields in request body                                      |
| 500 Server Error     | "Server error — check backend logs" + raw response                            |
| Network Error        | "Cannot reach backend — is the server running?"                               |
| CORS Error           | "CORS blocked — ensure backend allows requests from this origin"              |

---

## The "Polished" Checklist

The explorer is "polished" when:

- [ ] Opening the explorer takes < 500ms (lazy load the library)
- [ ] Auth token is always pre-populated when logged in
- [ ] Dark mode exactly matches the rest of the application
- [ ] All 169 endpoints are visible and searchable
- [ ] Path parameter inputs are shown for every parameterized endpoint
- [ ] The GET /health endpoint returns a successful response on the first try
- [ ] Sidebar scroll position persists when switching between endpoints
- [ ] Error responses are shown with the same formatting as success responses
- [ ] Response body is syntax-highlighted as JSON
- [ ] Long responses are scrollable, not overflowing the viewport
- [ ] The page title shows the selected endpoint (e.g., "Query — API Explorer")
- [ ] Mobile layout is usable (at minimum, not broken)
