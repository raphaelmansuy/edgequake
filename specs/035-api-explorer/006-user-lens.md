# SPEC-035 — EdgeQuake User Lens

**Lens:** EdgeQuake Users — Software Developer + Business User  
**Key Questions:**  
- What does a software developer need from an API explorer?  
- What does a business user need from an API explorer?  
- What does today's explorer prevent them from doing?  

---

## Two User Profiles

### Profile 1: The Software Developer User

**Who:** A software developer integrating EdgeQuake into their application, building automations, writing SDK code, or exploring the knowledge graph API.

**Context:**
- Has technical knowledge of REST APIs, JSON, HTTP methods
- Uses curl, Postman, or SDKs in daily work
- Expects to find all available endpoints with correct schemas
- Needs to understand auth mechanisms
- Wants to copy examples into their code

**Current Pain Journey:**

```
STEP 1: Dev opens API Explorer to find the PDF upload endpoint
→ PAIN: "POST /documents" is shown. But there's no PDF-specific upload listed.
→ ACTION: Dev searches docs, can't find it.
→ CONSEQUENCE: Dev misses the specialized PDF pipeline entirely.
             (POST /api/v1/pdf-documents IS the correct endpoint)

STEP 2: Dev finds POST /entities/merge in the explorer
→ PAIN: Clicks Execute with no body → 422 Unprocessable Entity
→ ACTION: Dev has to guess what fields are required.
→ CONSEQUENCE: Dev spends 30 min on trial and error.

STEP 3: Dev tries GET /documents/{id}
→ PAIN: Explorer sends "GET /documents/{id}" literally
→ RESULT: 404 Not Found
→ CONSEQUENCE: Dev thinks the endpoint is broken.

STEP 4: Dev needs to test with auth
→ PAIN: No way to set Authorization header in the explorer
→ ACTION: Switches to Postman, sets up token manually
→ CONSEQUENCE: Explorer is abandoned. Support ticket filed.
```

**Desired Developer Journey (after fix):**

```
STEP 1: Dev types "pdf" in search bar
→ RESULT: Sees PDF-specific endpoints immediately
           POST /api/v1/pdf-documents (with schema)
           GET  /api/v1/pdf-documents/{id}/status
           GET  /api/v1/pdf-documents/{id}/content
           POST /api/v1/pdf-documents/retry
→ OUTCOME: Discovery in 10 seconds

STEP 2: Dev clicks POST /entities/merge
→ RESULT: Schema shows required fields:
           source_entity: string (required) — "Entity to merge FROM"
           target_entity: string (required) — "Entity to merge INTO"
           merge_strategy: enum ["prefer_target", "prefer_source"] (optional)
→ OUTCOME: Correct request on first try

STEP 3: Dev tests GET /documents/{id}
→ RESULT: Input field "id (path parameter)" appears
           Dev types their document UUID
           Request sent to GET /documents/abc-123
→ OUTCOME: Correct response immediately

STEP 4: Auth pre-filled from session
→ RESULT: Authorization: Bearer eyJ... already in header
           Dev just clicks "Send"
→ OUTCOME: Explorer replaces Postman for daily exploration
```

---

### Profile 2: The Business User

**Who:** A business analyst, product manager, or enterprise decision-maker evaluating EdgeQuake, understanding its capabilities, or verifying integrations without writing code.

**Context:**
- Limited or no code experience
- Needs to understand "what can this API do?" at a functional level
- Uses the UI explorer as documentation, not just a test tool
- Cares about: What data does this return? Is it secure? What do I need to send?

**Current Pain Journey:**

```
STEP 1: Business analyst opens API Explorer to understand "what entities can I manage?"
→ PAIN: Sees 4 entity endpoints listed out of 11 actual entity endpoints
→ CONSEQUENCE: Analyst thinks entity management is limited.
              Misses: entity neighborhood, entity existence check, entity merge,
                      entity statistics, entity provenance, entity lineage

STEP 2: Analyst wants to understand the query response format
→ PAIN: No response schema shown. Static example not representative.
→ ACTION: Must ask developer to explain the response format
→ CONSEQUENCE: Lost autonomy; developer time wasted

STEP 3: Analyst wants to know "is this endpoint secure?"
→ PAIN: No auth information shown. No indication if authentication is needed.
→ CONSEQUENCE: Cannot assess security posture from the UI

STEP 4: Analyst asks "Can the API do X?" (e.g., conversation management)
→ PAIN: Conversations are NOT in the explorer (0 of 12 conversation endpoints shown)
→ CONSEQUENCE: Analyst answers "No, it can't" — wrong answer.
```

**Desired Business User Journey (after fix):**

```
STEP 1: Analyst opens API Explorer
→ RESULT: Sees logical groups with endpoint counts:
           Entities (11)  — Manage knowledge graph entities
           Query (5)      — RAG query execution
           Documents (15) — Document ingestion and management
           Conversations (12) — Chat history management
           Graph (8)      — Graph visualization and traversal
→ OUTCOME: Full capability picture in 30 seconds

STEP 2: Analyst browses "Conversations" section
→ RESULT: Sees all 12 conversation endpoints with descriptions
           "Create new conversation", "List messages", "Share conversation"
→ OUTCOME: Discovers feature they didn't know existed

STEP 3: Analyst checks POST /api/v1/query schema
→ RESULT: Sees response schema:
           query_results: QueryResult[]
             - content: string (the answer)
             - source_nodes: SourceReference[]
             - confidence: number (0.0 – 1.0)
→ OUTCOME: Understands data model without asking anyone

STEP 4: Analyst checks "is this secure?"
→ RESULT: Sees "🔒 Requires authentication — Bearer token"
           Sees "Rate limiting: 100 req/min per tenant"
→ OUTCOME: Security posture visible without reading code
```

---

## User Stories

### Developer User Stories

| ID     | As a developer, I want to...                                     | So that...                                     | Priority |
| ------ | ---------------------------------------------------------------- | ---------------------------------------------- | -------- |
| US-001 | Find all endpoints related to "pdf" with a single search         | I don't miss PDF-specific endpoints            | MUST     |
| US-002 | See the required and optional fields for any POST/PUT/PATCH body | I send correct requests on the first try       | MUST     |
| US-003 | Have my auth token pre-populated                                 | I can test protected endpoints without Postman | MUST     |
| US-004 | Input path parameter values (e.g., document ID)                  | Parameterized requests work correctly          | MUST     |
| US-005 | See the response schema for any endpoint                         | I can write client-side type definitions       | SHOULD   |
| US-006 | Copy a curl command for any request                              | I can share reproducible test cases            | SHOULD   |
| US-007 | See example responses from real calls                            | I understand the data shape                    | SHOULD   |
| US-008 | Use the explorer on mobile (at minimum, readable)                | Field testing while discussing API             | COULD    |

### Business User Stories

| ID     | As a business user, I want to...                | So that...                                | Priority |
| ------ | ----------------------------------------------- | ----------------------------------------- | -------- |
| US-010 | Browse all API capabilities by logical group    | I understand what EdgeQuake can do        | MUST     |
| US-011 | Read plain-language descriptions of endpoints   | I understand without technical background | MUST     |
| US-012 | See response schemas without executing requests | I understand data structures              | SHOULD   |
| US-013 | Know which endpoints require authentication     | I can assess security requirements        | SHOULD   |
| US-014 | Test a simple GET request (like /health)        | I verify the system is working            | SHOULD   |
| US-015 | Share a specific endpoint with a colleague      | I communicate API capabilities            | COULD    |

---

## Jobs-to-be-Done Matrix

| Job                            | Current Explorer           | New Explorer (Scalar)     |
| ------------------------------ | -------------------------- | ------------------------- |
| Discover all PDF endpoints     | ❌ (0 of 11 shown)          | ✅                         |
| Understand request body schema | ❌ (static example only)    | ✅ (full JSON Schema)      |
| Test with authentication       | ❌ (no auth support)        | ✅ (pre-populated token)   |
| Input path parameters          | ❌ (literal `{id}`)         | ✅ (dedicated input field) |
| See response schema            | ❌                          | ✅                         |
| Browse by API category         | ✅ (partial)                | ✅ (complete)              |
| Search endpoints               | ❌                          | ✅                         |
| Copy curl command              | ❌                          | ✅                         |
| See conversation endpoints     | ❌ (0 of 12 shown)          | ✅                         |
| See workspace management       | ❌ (0 of 8 shown)           | ✅                         |
| Understand auth requirements   | ❌                          | ✅                         |
| Test without writing code      | ⚠️ (GET only, 30 endpoints) | ✅ (all 169 endpoints)     |

---

## Feature Capability Map: What Users Discover (Before vs After)

### Before: 18% of the API is visible

```
Visible in current explorer:
  Health ●────────────────── 1/1   (100%)
  Auth ●────────────────────  2/4   (50%)
  Models ●──────────────────  4/5   (80%)
  Documents ●────────────── 4/15  (27%)
  Query ●────────────────────  1/5   (20%)
  Graph ●────────────────────  3/8   (38%)
  Entities ●──────────────── 5/11  (45%)
  Relationships ●──────────── 2/8   (25%)
  Pipeline ●──────────────── 1/4   (25%)
  Tenants ●───────────────── 4/6   (67%)
  Workspaces ●─────────────── 3/8   (38%)

INVISIBLE TO USERS (0% coverage):
  Conversations ────────────  0/12  (0%)
  Folders ──────────────────  0/4   (0%)
  Chat ─────────────────────  0/4   (0%)
  Users ────────────────────  0/6   (0%)
  API Keys ─────────────────  0/3   (0%)
  PDF Documents ────────────  0/11  (0%)
  Cost Tracking ────────────  0/5   (0%)
  Injections ───────────────  0/6   (0%)
  Lineage ──────────────────  0/8   (0%)
  OIDC ─────────────────────  0/4   (0%)
  Jobs (v2) ────────────────  0/4   (0%)
```

### After: 100% visible

```
All categories shown at 100% coverage.
New endpoints appear automatically as the API grows.
```

---

## User Acceptance Testing Scenarios

### UAT-001: Developer — Find PDF Upload Endpoint

```
Given: User is logged in and on /api-explorer
When:  User types "pdf" in the search bar
Then:  At least 5 endpoints containing "pdf" in path or description are shown
And:   POST endpoint for PDF upload is visible
```

### UAT-002: Developer — Test Authenticated Endpoint

```
Given: User is logged in
When:  User opens GET /api/v1/documents
And:   User clicks "Send"
Then:  Response is 200 OK (not 401)
And:   Response shows a list of documents
```

### UAT-003: Developer — Path Parameter Input

```
Given: User selects GET /api/v1/documents/{id}
When:  Explorer renders the endpoint
Then:  An input field for "id" is shown with label "id (path parameter)"
When:  User enters a valid document ID and clicks Send
Then:  Response is 200 (or 404 if ID not found) — NOT 404 "endpoint not found"
```

### UAT-004: Business User — Browse Capabilities

```
Given: User is on /api-explorer
When:  User scrolls the endpoint list
Then:  User can see "Conversations" section with multiple endpoints
And:   User can see "Cost" section with budget-related endpoints
And:   User can see "PDF" or "Documents" section with PDF-specific endpoints
```

### UAT-005: Both — Dark Mode

```
Given: User is on /api-explorer in dark mode (default)
When:  Explorer loads
Then:  Background is dark (matches sidebar/card background)
And:   Text is high-contrast (white/light gray)
And:   Method badges (GET=green, POST=blue, DELETE=red) are visible
And:   No jarring bright-white panels appear
```
