# Iteration 64 - CRITICAL: Multiple FEAT ID Collisions Discovered

## OBSERVE

### Systematic Code Scan Results

Scanned 100+ @implements annotations in `edgequake_webui/src/`. Found:

#### 1. **Duplicate FEAT IDs** (CRITICAL)

| FEAT ID  | Usage 1                                           | Usage 2                                   | Conflict |
| -------- | ------------------------------------------------- | ----------------------------------------- | -------- |
| FEAT0636 | `empty-state.tsx` - Empty state pattern           | `use-debounce.ts` - Debounce perf         | 🔴 YES   |
| FEAT0637 | `empty-state.tsx` - Contextual messaging          | `use-graph-expansion.ts` - Node expansion | 🔴 YES   |
| FEAT0638 | `use-graph-expansion.ts` - ForceAtlas2 layout     | `websocket-status.tsx` - Visual status    | 🔴 YES   |
| FEAT0639 | `use-graph-keyboard-navigation.ts` - Keyboard nav | `api-explorer.tsx` - API testing          | 🔴 YES   |
| FEAT0640 | `use-graph-keyboard-navigation.ts` - Focus mgmt   | `api-explorer.tsx` - Request viz          | 🔴 YES   |
| FEAT0801 | Auth (docs)                                       | `use-cost.ts` - Per-doc cost tracking     | 🔴 YES   |
| FEAT0803 | Auth (docs)                                       | `use-cost.ts` - Workspace cost summary    | 🔴 YES   |

#### 2. **Undocumented Features** (50+ features!)

FEAT07XX range (11 additional):

- FEAT0714: Camera fit-to-graph
- FEAT0715: Smooth camera animation
- FEAT0716: Louvain community detection
- FEAT0717: Community-based coloring
- FEAT0718: Source reference mapping
- FEAT0719: Entity categorization
- FEAT0720: UUID generation
- FEAT0721: Secure random fallback
- FEAT0722: Singleton WebSocket
- FEAT0723: Auto-reconnect
- FEAT0724-0726: WebSocket progress/heartbeat/subscription

FEAT07XX extensions (5 more):

- FEAT0727-0728: ✅ Already documented
- FEAT0729-0730: i18n support + browser detection
- FEAT0731-0732: Storage keys + migration
- FEAT0733: ✅ Already documented

FEAT06XX additions (20+ features):

- FEAT0628-0629: Folder operations + tenant switching
- FEAT0630-0650: Keyboard shortcuts, hydration, media queries, etc.

Other ranges:

- FEAT0401-0404: Conversation persistence
- FEAT0540-0541: Lineage tracking
- FEAT0740: Conversation panel
- FEAT0800: Theme support (docs collision!)
- FEAT0850-0852: Dashboard
- FEAT1050-1053: Onboarding tour

### Root Cause

**Lack of centralized FEAT ID registry during development.**

Multiple developers assigned IDs without checking existing allocations. Code grew to ~150+ features but features.md only tracked ~104.

## Impact

🔴 **CRITICAL**: Documentation is 33% incomplete
🔴 **CRITICAL**: 7 ID collisions make traceability impossible
🔴 **HIGH**: Cannot trust any FEAT reference in code or docs

## Decision

**Complete audit and reassignment required.**

Must:

1. Create master list of ALL code annotations
2. Resolve all collisions
3. Fill ALL gaps in features.md
4. Establish ID allocation process going forward
