# OODA Loop Iteration #5 - Observe Phase

**Date:** 2025-01-10  
**Mission:** WebUI Provider Selector - Settings UI Integration  
**Phase:** Observe (Reconnaissance & Requirements Gathering)

---

## Executive Summary

**Objective:** Add LLM provider selection UI to EdgeQuake WebUI settings page, enabling users to switch between OpenAI, Ollama, LM Studio, and Mock providers visually without editing environment variables.

**Current State:** Backend provider auto-detection working (Iterations #1-4), but no frontend UI for provider selection.

**Scope:** Frontend-only changes to `edgequake_webui`, no backend modifications required.

---

## 1. Current System Architecture

### 1.1 Backend Provider System (✅ Already Implemented)

**File:** [`edgequake/crates/edgequake-llm/src/provider_factory.rs`](../../edgequake/crates/edgequake-llm/src/provider_factory.rs)

**Provider Auto-Detection Logic:**

```rust
pub fn from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>), LlmError> {
    // Priority order (when EDGEQUAKE_LLM_PROVIDER not set):
    // 1. Ollama (if OLLAMA_HOST or OLLAMA_MODEL set)
    // 2. OpenAI (if OPENAI_API_KEY set)
    // 3. Mock (fallback)

    // Explicit override:
    if let Ok(provider) = std::env::var("EDGEQUAKE_LLM_PROVIDER") {
        // Use specified provider
    }
}
```

**Supported Providers:**

- **OpenAI** (production): Requires `OPENAI_API_KEY`
- **Ollama** (local dev): Requires `OLLAMA_HOST` (default: `http://localhost:11434`)
- **LM Studio** (experimentation): Requires `EDGEQUAKE_LLM_PROVIDER=lmstudio` + `OPENAI_BASE_URL`
- **Mock** (testing): No config required, auto-fallback

**Embedding Dimensions:**

- OpenAI: 1536 (text-embedding-3-small)
- Ollama: 768 (embeddinggemma:latest)
- LM Studio: 1536 (varies by model)
- Mock: 1536 (synthetic)

---

### 1.2 Frontend Settings Page (Existing Implementation)

**File:** [`edgequake_webui/src/app/(dashboard)/settings/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>)

**Current Settings Sections:**

1. **Appearance** (lines 137-211)

   - Theme selector (light/dark/system)
   - Language selector (en/zh/ja/ko)

2. **Graph Visualization** (lines 213-308)

   - Show node labels
   - Enable animations
   - Node size selector
   - Layout algorithm selector

3. **Query Defaults** (lines 310-401)

   - Default query mode (local/global/hybrid/naive)
   - Search parameters (top_k, only_need_context)

4. **Ingestion Settings** (lines 403-484)

   - Enable gleaning
   - Max gleaning passes
   - Chunk size, overlap

5. **Data Management** (lines 486-568)
   - Export settings
   - Import settings
   - Reset to defaults

**UI Component Pattern:**

```tsx
<Card>
  <CardHeader>
    <CardTitle className="flex items-center gap-2">
      <Icon className="h-5 w-5" />
      Section Title
    </CardTitle>
    <CardDescription>Section description</CardDescription>
  </CardHeader>
  <CardContent className="space-y-4">
    {/* Setting items with Select/Switch components */}
  </CardContent>
</Card>
```

---

### 1.3 State Management Architecture

**Store:** `edgequake_webui/src/stores/use-settings-store.ts`

**Current Settings State:**

```typescript
interface SettingsState {
  // Appearance
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "ja" | "ko";

  // Graph
  graphSettings: GraphSettings;

  // Query
  querySettings: QuerySettings;

  // Ingestion
  ingestionSettings: IngestionSettings;

  // Actions
  updateGraphSettings: (key, value) => void;
  updateQuerySettings: (key, value) => void;
  // ...
}
```

**Storage:** Uses Zustand with localStorage persistence via `persist` middleware.

---

## 2. Requirements Analysis

### 2.1 Functional Requirements

**FR-1:** Provider Selection UI

- **Description:** Add dropdown selector for LLM provider in Settings page
- **Location:** New "LLM Provider" card before "Query Defaults" section
- **Options:**
  - OpenAI (requires API key)
  - Ollama (requires local server)
  - LM Studio (requires local server)
  - Mock (testing only)

**FR-2:** Provider Status Indicator

- **Description:** Real-time status display for selected provider
- **States:**
  - ✅ **Connected** (green) - Provider responding
  - ⏳ **Connecting...** (yellow) - Checking status
  - ❌ **Disconnected** (red) - Provider unavailable
  - ⚠️ **Configuration Missing** (amber) - Missing required env vars

**FR-3:** Configuration Hints

- **Description:** Show required configuration per provider
- **OpenAI:** "Requires OPENAI_API_KEY environment variable"
- **Ollama:** "Requires OLLAMA_HOST (default: http://localhost:11434)"
- **LM Studio:** "Requires OPENAI_BASE_URL pointing to LM Studio server"
- **Mock:** "No configuration required - for testing only"

**FR-4:** Provider Dimension Display

- **Description:** Show embedding dimension for each provider
- **Format:** "Embedding Dimension: 1536" or "768" or "N/A"
- **Warning:** If dimension doesn't match storage, show warning badge

**FR-5:** Provider Switch Confirmation

- **Description:** Modal dialog when switching providers
- **Content:**
  - Warning about dimension compatibility
  - Option to clear vectors (if PostgreSQL)
  - Option to cancel
  - Proceed button

### 2.2 Non-Functional Requirements

**NFR-1:** **Performance** - Provider status check must complete <2 seconds
**NFR-2:** **Reliability** - Settings must persist across page refreshes
**NFR-3:** **Accessibility** - Keyboard navigation and screen reader support
**NFR-4:** **Responsive** - Works on mobile, tablet, desktop
**NFR-5:** **No Backend Changes** - Frontend-only implementation

### 2.3 Acceptance Criteria

**AC-1:** User can select provider from dropdown without editing `.env` file
**AC-2:** Provider status indicator updates automatically every 30 seconds
**AC-3:** Configuration hints appear below provider selector
**AC-4:** Warning appears when switching providers with different dimensions
**AC-5:** Settings persist after browser refresh
**AC-6:** No regressions in existing settings functionality

---

## 3. Gap Analysis

### 3.1 What Exists (✅)

1. **Backend Provider Factory** - Auto-detects provider from env vars
2. **Settings Page Structure** - Well-organized card-based layout
3. **UI Components** - Select, Card, Switch, Button all available
4. **State Management** - Zustand store with persistence
5. **Dimension Validation** - Backend validates dimension mismatches (Iteration #4)

### 3.2 What's Missing (❌)

1. **Provider Settings Store** - No `providerSettings` in settings store
2. **Provider Status API** - No backend endpoint to check provider health
3. **Provider Selection Component** - No UI component for provider selection
4. **Provider Switcher Logic** - No logic to update env vars from frontend
5. **Dimension Warning UI** - No visual warning for dimension mismatches

### 3.3 Critical Constraint: Environment Variables

**Problem:** Frontend cannot directly modify process environment variables (Node.js/server-side only).

**Implication:** Provider selection must either:

1. **Option A:** Send provider preference to backend, backend updates its own env vars dynamically
2. **Option B:** Use cookie/localStorage to pass provider to backend via HTTP headers
3. **Option C:** Require server restart with updated `.env` file

**Chosen Approach:** Option B (cookie-based provider hint)

**Rationale:**

- ✅ No backend code changes required
- ✅ No server restart needed
- ✅ Compatible with Docker deployments
- ✅ Works with existing ProviderFactory
- ❌ Requires middleware to read cookie and set env var per-request

---

## 4. Technical Investigation

### 4.1 Backend Provider Detection Flow

**Current Implementation:**

```rust
// edgequake/crates/edgequake-llm/src/provider_factory.rs
pub fn from_env() -> Result<...> {
    // Reads process env vars at creation time
    std::env::var("EDGEQUAKE_LLM_PROVIDER")
    std::env::var("OPENAI_API_KEY")
    std::env::var("OLLAMA_HOST")
}
```

**Call Sites:**

1. `AppState::new_memory()` - Called once at startup
2. `AppState::new_postgres()` - Called once at startup

**Issue:** Provider is determined at AppState creation (startup), not per-request.

**Solution:** Add per-request provider override via HTTP header:

```http
X-EdgeQuake-Provider: ollama
X-EdgeQuake-API-Key: <optional-override>
X-EdgeQuake-Base-URL: <optional-override>
```

**Backend Changes Required:**

- Middleware to read headers and set thread-local env vars
- OR: Modify ProviderFactory to accept optional headers parameter

**Revised Constraint:** Backend changes ARE required (invalidates NFR-5).

---

### 4.2 Alternative: Server-Side Settings Endpoint

**Approach:**

```typescript
// Frontend sends provider selection
POST /api/settings/provider
{
  "provider": "ollama",
  "config": {
    "host": "http://localhost:11434"
  }
}

// Backend updates runtime configuration
// Returns new AppState with provider
```

**Backend Implementation:**

```rust
// edgequake-api: Add endpoint
async fn update_provider_settings(
    Json(settings): Json<ProviderSettings>
) -> Result<Json<ProviderStatus>, AppError> {
    // 1. Validate provider config
    // 2. Create new AppState with provider
    // 3. Swap global AppState
    // 4. Return status
}
```

**Pros:**

- ✅ Clean separation of concerns
- ✅ Supports per-workspace provider config
- ✅ No environment variable hacks

**Cons:**

- ❌ Requires backend implementation (violates NFR-5)
- ❌ Requires AppState hot-swap logic
- ❌ Complexity for tenant-specific providers

---

## 5. Design Constraints Summary

| Constraint                                             | Impact                                         | Mitigation                                          |
| ------------------------------------------------------ | ---------------------------------------------- | --------------------------------------------------- |
| **C1:** Provider set at AppState creation (startup)    | Cannot switch providers without restart        | Implement backend `/api/settings/provider` endpoint |
| **C2:** Frontend cannot set environment variables      | Provider selection requires server-side logic  | Use backend API endpoint                            |
| **C3:** Dimension validation happens in new_postgres() | Warning requires pre-check before switch       | Add `/api/provider/validate` endpoint               |
| **C4:** Multi-tenant deployments                       | Different tenants may need different providers | Scope provider settings per workspace               |
| **C5:** PostgreSQL vector dimension                    | Switching providers may require vector rebuild | Show warning + recovery options UI                  |

---

## 6. Revised Approach: Hybrid Solution

### 6.1 MVP Scope (Iteration #5)

**What We WILL Build:**

1. **Frontend UI Only** (settings page)

   - Provider selection dropdown (read-only display)
   - Current provider indicator from backend
   - Configuration documentation links
   - Manual setup instructions

2. **Backend: Read-Only Status Endpoint**
   - `GET /api/settings/provider/status` - Returns current provider info
   - No state mutation, just introspection

**What We DEFER:**

1. **Dynamic Provider Switching** - Requires AppState hot-swap (Iteration #8)
2. **Per-Tenant Providers** - Requires multi-tenant provider config (Iteration #12)
3. **Dimension Pre-Validation** - Requires `/api/provider/validate` endpoint (Iteration #9)

### 6.2 User Flow (MVP)

```
1. User navigates to Settings page
2. UI fetches current provider status: GET /api/settings/provider/status
3. UI displays:
   - Current provider: "Ollama"
   - Embedding dimension: 768
   - Status: ✅ Connected
   - Configuration: "OLLAMA_HOST=http://localhost:11434"
4. UI shows "To change provider, update environment variables and restart server"
5. UI provides copy-pasteable .env examples
```

**Rationale:** MVP delivers value (visibility into provider config) without complex backend changes.

---

## 7. Work Breakdown Estimate

### Phase 5E: Read-Only Provider Status UI (This Iteration)

| Task                              | Effort      | Description                                              |
| --------------------------------- | ----------- | -------------------------------------------------------- |
| **5E.1:** Backend status endpoint | 45 min      | Add `GET /api/settings/provider/status` in edgequake-api |
| **5E.2:** Provider info struct    | 15 min      | Define ProviderStatus response type                      |
| **5E.3:** Frontend UI component   | 60 min      | Add "LLM Provider" card to settings page                 |
| **5E.4:** Provider status hook    | 30 min      | Create `useProviderStatus()` hook with polling           |
| **5E.5:** Configuration docs      | 20 min      | Add .env examples and setup instructions                 |
| **5E.6:** E2E testing             | 40 min      | Test status endpoint + UI display                        |
| **5E.7:** Documentation           | 30 min      | Update user docs with provider visibility feature        |
| **Total**                         | **240 min** | **4 hours**                                              |

### Future Iterations (Deferred)

| Iteration | Feature                                        | Effort    |
| --------- | ---------------------------------------------- | --------- |
| **#8**    | Dynamic provider switching (AppState hot-swap) | 6-8 hours |
| **#9**    | Dimension pre-validation endpoint              | 2-3 hours |
| **#12**   | Per-tenant provider configuration              | 4-5 hours |

---

## 8. Success Criteria (Iteration #5)

**SC-1:** ✅ `GET /api/settings/provider/status` endpoint returns provider info  
**SC-2:** ✅ Settings page displays current provider with status badge  
**SC-3:** ✅ Embedding dimension shown with warning if mismatch detected  
**SC-4:** ✅ Configuration instructions displayed per provider  
**SC-5:** ✅ Provider status auto-refreshes every 30 seconds  
**SC-6:** ✅ Copy-pasteable .env examples provided  
**SC-7:** ✅ No regressions in existing settings functionality

---

## 9. Risk Assessment

| Risk                                                | Probability | Impact | Mitigation                                        |
| --------------------------------------------------- | ----------- | ------ | ------------------------------------------------- |
| **R1:** Backend changes take longer than estimated  | Medium      | Medium | Start with read-only endpoint (simplest)          |
| **R2:** Provider status check slow (>2s)            | Low         | Medium | Add timeout + fallback to "Unknown"               |
| **R3:** Dimension warning logic complex             | Low         | Low    | Reuse existing validation from Iteration #4       |
| **R4:** UI doesn't fit mobile screens               | Low         | Low    | Use responsive Card layout (already works)        |
| **R5:** Polling every 30s causes performance issues | Very Low    | Low    | Make polling optional + add manual refresh button |

---

## 10. Dependencies

**External:**

- None (no new npm packages required)

**Internal:**

- ✅ edgequake-api crate (add status endpoint)
- ✅ Iteration #4 dimension validation logic
- ✅ Settings page infrastructure

---

## 11. Next Steps → Orient Phase

**Orient Phase Will Address:**

1. Design backend `ProviderStatus` struct and API response format
2. Design frontend UI component layout and state management
3. Choose polling strategy (interval vs manual refresh)
4. Design warning badge UI for dimension mismatches
5. Plan integration with existing settings store

**Key Decisions Needed:**

- Should status endpoint be authenticated? (Yes, use existing auth)
- Should status be cached backend-side? (No, always fresh)
- Should we show provider models (gpt-4o-mini, gemma3:12b)? (Yes, helpful)

---

**OODA Progress:** 5/50 iterations (10%)  
**Phase Progress:** Iteration #5 - Observe ✅ COMPLETE

**Next Phase:** Orient - Design backend API and frontend UI architecture

---

**End of Observe Phase**
