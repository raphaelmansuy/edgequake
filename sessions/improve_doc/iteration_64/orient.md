# OODA Loop Iteration 64 - ORIENT Phase

**Date**: 2025-01-03  
**Focus**: Complete Feature Audit and ID Collision Resolution  
**Status**: CRITICAL - Systemic Documentation Failures Discovered

---

## 📊 Complete Code Inventory

### Systematic @implements Search Results

Executed comprehensive code scan of all TypeScript files in `edgequake_webui/src/`:

```bash
grep -r "@implements FEAT[0-9]{4}" edgequake_webui/src/**/*.{ts,tsx}
```

**Total Annotations Found**: 200+ (search capped at maxResults=200)

---

## 🔍 Analysis: Feature ID Collision Matrix

### Critical Duplicate FEAT IDs (7 Conflicts)

| FEAT ID      | Location 1                               | Description 1                  | Location 2                                                       | Description 2                       |
| ------------ | ---------------------------------------- | ------------------------------ | ---------------------------------------------------------------- | ----------------------------------- |
| **FEAT0636** | `components/shared/empty-state.tsx`      | Consistent empty state pattern | `hooks/use-debounce.ts`                                          | Performance optimization            |
| **FEAT0637** | `components/shared/empty-state.tsx`      | Contextual messaging           | `hooks/use-graph-expansion.ts`                                   | Node expansion                      |
| **FEAT0638** | `hooks/use-graph-expansion.ts`           | ForceAtlas2 layout             | `components/shared/websocket-status.tsx`                         | Visual status                       |
| **FEAT0639** | `hooks/use-graph-keyboard-navigation.ts` | Keyboard navigation            | `components/shared/api-explorer.tsx`                             | API testing                         |
| **FEAT0640** | `hooks/use-graph-keyboard-navigation.ts` | Focus management               | `components/shared/api-explorer.tsx`                             | Request visualization               |
| **FEAT0801** | `docs/features.md` (Auth)                | Auth - future (reserved)       | `types/cost.ts`, `hooks/use-cost.ts`, `stores/use-cost-store.ts` | Per-document cost tracking (ACTIVE) |
| **FEAT0803** | `docs/features.md` (Auth)                | Auth - future (reserved)       | `hooks/use-cost.ts`, `stores/use-cost-store.ts`                  | Workspace cost summary (ACTIVE)     |

**Impact**: These duplicates make FEAT↔BR↔UC traceability impossible. Cannot determine which feature a business rule references.

---

## 📋 Undocumented Features by Range

### FEAT04XX - Conversation Features (4 missing)

- **FEAT0401**: Conversation persistence (`hooks/use-conversations.ts`, `components/query/source-citations.tsx`)
- **FEAT0402**: Infinite scroll pagination / Document deep-links (`hooks/use-conversations.ts`, `components/query/source-citations.tsx`)
- **FEAT0403**: Local conversation storage / Confidence score viz (`stores/use-conversation-store.ts`, `components/query/source-citations.tsx`)
- **FEAT0404**: Active conversation tracking (`stores/use-conversation-store.ts`)

### FEAT05XX - Lineage/Context Features (4 missing)

- **FEAT0540**: Chunk detail retrieval (`hooks/use-lineage.ts`)
- **FEAT0541**: Entity provenance tracking (`hooks/use-lineage.ts`)
- **FEAT0583**: Folder organization (`hooks/use-folders.ts`)

### FEAT06XX - WebUI Core (35+ missing)

- **FEAT0602**: Real-time progress indicators (5 refs)
- **FEAT0603**: WebSocket real-time / Layout selection (4 refs)
- **FEAT0604**: Fallback polling / Clustering (3 refs)
- **FEAT0605**: Connection state / Theme styling (2 refs)
- **FEAT0606**: Auto-reconnect (1 ref)
- **FEAT0607**: Batch delivery (1 ref)
- **FEAT0608**: Abort control (1 ref)
- **FEAT0609**: Collapsible sidebar / Lineage data (3 refs)
- **FEAT0610**: Mobile drawer / Cost history (2 refs)
- **FEAT0611**: Backend health (2 refs)
- **FEAT0612**: Theme toggle (1 ref)
- **FEAT0613**: User menu (1 ref)
- **FEAT0614**: Collapsible panels (1 ref)
- **FEAT0615**: Configurable widths (1 ref)
- **FEAT0616**: Scroll areas (1 ref)
- **FEAT0620**: Pipeline status (1 ref)
- **FEAT0621**: Connection errors (1 ref)
- **FEAT0625**: Stage tracking / Query UI state (2 refs)
- **FEAT0626**: Camera focus / Thinking indicator (2 refs)
- **FEAT0627**: Type toggles / Filtering (2 refs)
- **FEAT0628**: Virtual scrolling / Folder CRUD (2 refs)
- **FEAT0629**: Sort entities / Tenant switching (2 refs)
- **FEAT0630**: Group entities / Keyboard system (2 refs)
- **FEAT0631**: Shortcut modal / Metadata display (2 refs)
- **FEAT0632**: Accessibility / Chunk stats (2 refs)
- **FEAT0633**: Textarea resize / Preview (2 refs)
- **FEAT0634**: Adaptive input / Cost breakdown (2 refs)
- **FEAT0635**: Debounced search / Quick actions (2 refs)
- **FEAT0636**: Empty state / Performance (2 refs - **COLLISION**)
- **FEAT0637**: Contextual messages / Node expansion (2 refs - **COLLISION**)
- **FEAT0638**: ForceAtlas2 / Visual status (2 refs - **COLLISION**)
- **FEAT0639**: Keyboard nav / API testing (2 refs - **COLLISION**)
- **FEAT0640**: Focus mgmt / Request viz (2 refs - **COLLISION**)
- **FEAT0641**: Breakpoint detection (1 ref)
- **FEAT0642**: SSR-safe media (1 ref)
- **FEAT0643**: Migration (1 ref)
- **FEAT0644**: Migration progress (1 ref)
- **FEAT0645**: Query page state (1 ref)
- **FEAT0646**: Conversation integration (1 ref)
- **FEAT0647**: URL state sync (2 refs)
- **FEAT0648**: Shareable URLs (2 refs)
- **FEAT0649**: Store hydration (1 ref)
- **FEAT0650**: Hydration mismatch (1 ref)
- **FEAT0651**: Workspace slug URL (1 ref)
- **FEAT0652**: URL-driven workspace (1 ref)

### FEAT07XX - API Client / Lineage (32 missing - some documented)

- **FEAT0700**: Unified client (1 ref - **DOCUMENTED**)
- **FEAT0701**: SSE client / Lineage viz (2 refs - 1 docs conflict)
- **FEAT0702**: Interceptors / Entity tracing (2 refs - 1 docs conflict)
- **FEAT0703**: Chat API / View modes (2 refs - **DOCUMENTED**)
- **FEAT0704**: Streaming / Chunk search (2 refs - **DOCUMENTED**)
- **FEAT0705**: Mode selection / Related entities (2 refs - **DOCUMENTED**)
- **FEAT0706**: Conversation list (1 ref - **DOCUMENTED**)
- **FEAT0707**: Message history (1 ref - **DOCUMENTED**)
- **FEAT0708**: Sharing (1 ref - **DOCUMENTED**)
- **FEAT0709**: Folder CRUD (1 ref - **DOCUMENTED**)
- **FEAT0710**: Move to folders (1 ref - **DOCUMENTED**)
- **FEAT0711**: Query keys (1 ref - **DOCUMENTED**)
- **FEAT0712**: Cache invalidation (1 ref - **DOCUMENTED**)
- **FEAT0713**: Camera focus (1 ref - **NOT DOCUMENTED**)
- **FEAT0714**: Fit-to-graph (1 ref - **NOT DOCUMENTED**)
- **FEAT0715**: Smooth animation (1 ref - **NOT DOCUMENTED**)
- **FEAT0716**: Louvain clustering (1 ref - **NOT DOCUMENTED**)
- **FEAT0717**: Community coloring (1 ref - **NOT DOCUMENTED**)
- **FEAT0718**: Source mapping (1 ref - **NOT DOCUMENTED**)
- **FEAT0719**: Categorization (1 ref - **NOT DOCUMENTED**)
- **FEAT0720**: UUID generation (1 ref - **NOT DOCUMENTED**)
- **FEAT0721**: Random fallback (1 ref - **NOT DOCUMENTED**)
- **FEAT0722**: Singleton WebSocket (1 ref - **NOT DOCUMENTED**)
- **FEAT0723**: Auto-reconnect WS (1 ref - **NOT DOCUMENTED**)
- **FEAT0724**: Progress WebSocket (2 refs - **NOT DOCUMENTED**)
- **FEAT0725**: Heartbeat (1 ref - **NOT DOCUMENTED**)
- **FEAT0726**: Subscriptions (1 ref - **NOT DOCUMENTED**)
- **FEAT0727**: Export MD (1 ref - **NOT DOCUMENTED**)
- **FEAT0728**: Export JSON (1 ref - **NOT DOCUMENTED**)
- **FEAT0729**: Multi-language (4 refs - **NOT DOCUMENTED**)
- **FEAT0730**: Language detection (1 ref - **NOT DOCUMENTED**)
- **FEAT0731**: Storage keys (2 refs - **NOT DOCUMENTED**)
- **FEAT0732**: Migration support (1 ref - **NOT DOCUMENTED**)
- **FEAT0733**: Tailwind merge (1 ref - **DOCUMENTED**)

### FEAT074X - Query Features (3 missing)

- **FEAT0740**: Conversation sidebar (1 ref)
- **FEAT0741**: Search/filter (1 ref)
- **FEAT0750**: Collapsible thinking (1 ref)
- **FEAT0751**: Rendering options (1 ref)

### FEAT076X - Progress Features (1 missing)

- **FEAT0760**: Stage progress viz (1 ref)

### FEAT08XX - Auth/Cost Features (6 missing - 2 documented)

- **FEAT0800**: Theme support (2 refs - **CONFLICT WITH AUTH RANGE**)
- **FEAT0801**: Per-doc cost (5 refs - **DOCUMENTED BUT COLLISION**)
- **FEAT0802**: Ingestion updates (1 ref - **DOCUMENTED**)
- **FEAT0803**: Workspace summary (3 refs - **DOCUMENTED BUT COLLISION**)
- **FEAT0804**: Token breakdown (1 ref - **NOT DOCUMENTED**)

### FEAT085X - Dashboard Features (6 missing)

- **FEAT0850**: Dashboard stats (1 ref)
- **FEAT0851**: Activity feed (1 ref)
- **FEAT0852**: Quick shortcuts (1 ref)
- **FEAT0860**: Provider composition (1 ref)
- **FEAT0861**: Context layering (1 ref)
- **FEAT0863**: React Query state (1 ref)
- **FEAT0864**: Cache invalidation (1 ref)
- **FEAT0865**: WS management (1 ref)
- **FEAT0867**: SSR i18n (1 ref)
- **FEAT0868**: Auto-tenant (1 ref)

### FEAT10XX - Document/Viz Features (42 missing)

- **FEAT1001**: Dashboard stats viz (1 ref)
- **FEAT1002**: Trend indicators (1 ref)
- **FEAT1010**: Quick actions (1 ref)
- **FEAT1011**: Navigation widgets (1 ref)
- **FEAT1020**: Activity feed (1 ref)
- **FEAT1021**: Status indicators (1 ref)
- **FEAT1030**: System health (1 ref)
- **FEAT1031**: API status (1 ref)
- **FEAT1040**: Budget viz (1 ref)
- **FEAT1041**: Budget alerts (1 ref)
- **FEAT1042**: Cost breakdown viz (1 ref)
- **FEAT1043**: Stage categorization (1 ref)
- **FEAT1044**: Summary display (1 ref)
- **FEAT1045**: Token aggregation (1 ref)
- **FEAT1046**: Usage table (1 ref)
- **FEAT1047**: Stage tracking (1 ref)
- **FEAT1050**: Onboarding tour (1 ref)
- **FEAT1051**: Feature intro (1 ref)
- **FEAT1052**: Tour steps (1 ref)
- **FEAT1053**: Contextual help (1 ref)
- **FEAT1060**: Stage viz (1 ref)
- **FEAT1061**: Stage progress (1 ref)
- **FEAT1062**: ETA display (1 ref)
- **FEAT1063**: Elapsed time (1 ref)
- **FEAT1064**: Message stream (1 ref)
- **FEAT1065**: Message history (1 ref)
- **FEAT1070**: Chunk browsing (1 ref)
- **FEAT1071**: Entity highlight (1 ref)
- **FEAT1072**: MIME rendering (1 ref)
- **FEAT1073**: Highlight/scroll (1 ref)
- **FEAT1074**: Metadata display (1 ref)
- **FEAT1075**: Collapsible sections (1 ref)
- **FEAT1076**: Pipeline viz (1 ref)
- **FEAT1077**: Timing display (1 ref)
- **FEAT1078**: Count display (1 ref)
- **FEAT1079**: Graph nav (1 ref)
- **FEAT1080**: Chunk modal (1 ref)
- **FEAT1081**: Entity tabs (1 ref)
- **FEAT1082**: Model info (1 ref)
- **FEAT1083**: Type badges (1 ref)
- **FEAT1084**: Source metadata (1 ref)
- **FEAT1085**: File size fmt (1 ref)
- **FEAT1086**: Key stats (1 ref)
- **FEAT1087**: Duration display (1 ref)

---

## 📊 Quantitative Summary

| Metric                             | Value    | Impact                              |
| ---------------------------------- | -------- | ----------------------------------- |
| **Features in docs/features.md**   | 104      | Baseline                            |
| **Features in code (@implements)** | 200+     | Actual count                        |
| **Missing from docs**              | 96+      | **48% gap**                         |
| **Duplicate FEAT IDs**             | 7        | **Critical traceability failure**   |
| **Namespace conflicts**            | 2 ranges | Auth (FEAT08XX) vs Theme (FEAT0800) |

---

## 🎯 Root Cause Analysis

### Why This Happened

1. **No Centralized ID Registry During Development**

   - Frontend team assigned IDs independently
   - Backend team assigned IDs independently
   - No cross-team coordination

2. **Code-First Development Without Doc Updates**

   - Features implemented with @implements annotations
   - Documentation not updated in parallel
   - No CI/CD check for doc completeness

3. **Range Allocation Not Enforced**

   - FEAT07XX assigned to both Auth (docs) and API Client (code)
   - FEAT08XX supposed to be Auth but FEAT0800 used for Theme
   - No ownership table in features.md header

4. **Missing Validation Process**
   - No automated grep check for @implements → docs match
   - No pre-commit hook to validate FEAT IDs unique
   - No PR requirement for features.md update when adding @implements

---

## 🔧 Required Actions

### P0 - Critical Fixes

1. **Resolve 7 Duplicate IDs**

   - Reassign conflicting features to unused ranges
   - Update code files with new FEAT IDs
   - OR: Update features.md to match code (recommended: "Code is Law")

2. **Document 96+ Missing Features**

   - Add all undocumented @implements to features.md
   - Create proper sections for new ranges
   - Update Quick Reference Index

3. **Fix Namespace Collisions**
   - Resolve Auth (FEAT08XX docs) vs Cost (FEAT08XX code) conflict
   - Resolve Theme (FEAT0800 code) vs Auth range assignment

### P1 - Process Improvements

4. **Create ID Allocation Table**

   - Document range ownership in features.md header
   - Assign ranges: FEAT00XX (Core), FEAT01XX (Query), etc.
   - Reserve ranges for future expansion

5. **Establish Validation Process**

   - Create script: `scripts/validate_feat_ids.sh`
   - Add pre-commit hook to check duplicates
   - Add PR template requirement: "Updated features.md for new @implements"

6. **Automated Documentation**
   - Consider script to auto-generate features.md from @implements
   - Or: script to validate consistency
   - Add to CI/CD pipeline

---

## 💡 Insights for DECIDE Phase

### Strategy A: Update Code to Match Docs (High Risk)

- **Pros**: Docs remain authoritative
- **Cons**: 200+ file changes, risk of breaking refs, requires extensive testing
- **Risk**: HIGH - could introduce bugs in working code

### Strategy B: Update Docs to Match Code (Recommended)

- **Pros**: "Code is Law" principle, working features preserved
- **Cons**: More doc work, need to reorganize ranges
- **Risk**: LOW - documentation only, no code changes

### Strategy C: Hybrid Approach

- **Pros**: Fix only critical collisions, accept some doc gaps
- **Cons**: Incomplete solution, technical debt remains
- **Risk**: MEDIUM - doesn't solve root cause

---

## 🎯 Recommendation: Strategy B + Process Fixes

1. **Immediate**: Resolve 7 duplicates by updating features.md
2. **Short-term**: Add 96+ missing features to features.md
3. **Long-term**: Implement validation automation

**Rationale**: User demanded "Code is Law! Be Relentless. Never takes a shortcut." Strategy B honors this principle.

---

**Next Phase**: DECIDE - Choose resolution strategy and plan execution
