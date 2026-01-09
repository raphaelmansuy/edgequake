# Orient - Iteration 58

## Analysis of Observations

### Root Cause Analysis

```
┌─────────────────────────────────────────────────────────────────┐
│                 DOCUMENTATION GAPS HIERARCHY                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  CRITICAL: FEAT ID CONFLICTS                             │   │
│   │  - FEAT0601 used for TWO different features              │   │
│   │  - Breaks traceability and code references               │   │
│   └──────────────────────┬──────────────────────────────────┘   │
│                          │                                       │
│   ┌──────────────────────▼──────────────────────────────────┐   │
│   │  HIGH: 16 MISSING WebUI FEATURES                         │   │
│   │  - features.md only has FEAT0601-0604                    │   │
│   │  - Code references FEAT0605-0620                         │   │
│   └──────────────────────┬──────────────────────────────────┘   │
│                          │                                       │
│   ┌──────────────────────▼──────────────────────────────────┐   │
│   │  MEDIUM: STORE DOCUMENTATION SPARSE                      │   │
│   │  - Only mentioned in passing in 0014-webui-state.md      │   │
│   │  - No mapping between stores and features                │   │
│   └──────────────────────┬──────────────────────────────────┘   │
│                          │                                       │
│   ┌──────────────────────▼──────────────────────────────────┐   │
│   │  LOWER: HOOKS NEED CATALOG                               │   │
│   │  - 20 hooks with no documentation                        │   │
│   │  - Important for developer onboarding                    │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Feature ID Conflict Resolution

**Conflict:**

- `features.md` → FEAT0601 = "Document Upload UI"
- `use-graph-store.ts` → @implements FEAT0601 = "Knowledge Graph Visualization"

**Resolution Options:**

| Option | Description                        | Impact          | Recommendation   |
| ------ | ---------------------------------- | --------------- | ---------------- |
| A      | Keep features.md, update code      | Modify 2+ files | ❌ Higher effort |
| B      | Keep code refs, update features.md | Modify 1 file   | ✅ Simpler       |
| C      | Renumber all FEAT06XX              | Massive change  | ❌ Too risky     |

**Decision:** Option B - Update features.md to match code

- FEAT0601 → Document Upload UI (rename to FEAT0605)
- FEAT0601 → Knowledge Graph Visualization (keep for code compatibility)

### Proposed Feature Registry (FEAT06XX)

| ID       | Name                          | Module          | Status | Stores/Hooks           |
| -------- | ----------------------------- | --------------- | ------ | ---------------------- |
| FEAT0601 | Knowledge Graph Visualization | edgequake_webui | ✅     | use-graph-store        |
| FEAT0602 | Chat Query Interface          | edgequake_webui | ✅     | use-query-store        |
| FEAT0603 | Streaming Response Display    | edgequake_webui | ✅     | use-query-store        |
| FEAT0604 | Query Mode Selector           | edgequake_webui | ✅     | use-query-ui-store     |
| FEAT0605 | Document Upload Interface     | edgequake_webui | ✅     | use-ingestion-store    |
| FEAT0606 | Workspace Switcher            | edgequake_webui | ✅     | use-tenant-store       |
| FEAT0607 | Entity Type Filter            | edgequake_webui | ✅     | use-graph-store        |
| FEAT0608 | Graph Bookmark Manager        | edgequake_webui | ✅     | use-graph-store        |
| FEAT0609 | Conversation Persistence      | edgequake_webui | ✅     | use-conversation-store |
| FEAT0610 | Cost Tracking Display         | edgequake_webui | ✅     | use-cost-store         |
| FEAT0611 | Ingestion Progress Monitor    | edgequake_webui | ✅     | use-ingestion-progress |
| FEAT0612 | Keyboard Navigation           | edgequake_webui | ✅     | use-keyboard-shortcuts |
| FEAT0613 | Dark/Light Theme              | edgequake_webui | ✅     | use-settings-store     |
| FEAT0614 | Multi-Language (i18n)         | edgequake_webui | ✅     | i18next                |
| FEAT0615 | Source Citation Links         | edgequake_webui | ✅     | use-lineage            |
| FEAT0616 | Entity Search (MiniSearch)    | edgequake_webui | ✅     | use-graph-store        |
| FEAT0617 | User Preference Persistence   | edgequake_webui | ✅     | use-settings-store     |
| FEAT0618 | Graph Layout Settings         | edgequake_webui | ✅     | use-settings-store     |
| FEAT0619 | Ingestion Quality Config      | edgequake_webui | ✅     | use-settings-store     |
| FEAT0620 | Query Result Export           | edgequake_webui | 🔧     | -                      |

### Store-Feature Mapping

```
┌──────────────────────────┬───────────────────────────────────┐
│        Zustand Store     │         Features Implemented      │
├──────────────────────────┼───────────────────────────────────┤
│ use-graph-store          │ FEAT0601, FEAT0607, FEAT0608,     │
│                          │ FEAT0616                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-query-store          │ FEAT0602, FEAT0603                │
├──────────────────────────┼───────────────────────────────────┤
│ use-query-ui-store       │ FEAT0604                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-settings-store       │ FEAT0613, FEAT0617, FEAT0618,     │
│                          │ FEAT0619                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-ingestion-store      │ FEAT0605                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-tenant-store         │ FEAT0606                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-conversation-store   │ FEAT0609                          │
├──────────────────────────┼───────────────────────────────────┤
│ use-cost-store           │ FEAT0610                          │
└──────────────────────────┴───────────────────────────────────┘
```

### Impact Assessment

| Change                           | Files Affected         | Risk Level |
| -------------------------------- | ---------------------- | ---------- |
| Update features.md FEAT06XX      | 1                      | LOW        |
| Add hook catalog                 | 1 (new file or update) | LOW        |
| Update WebUI docs with FEAT refs | 7                      | LOW        |
| Add store-feature mapping        | 1                      | LOW        |

### Priority Ranking

1. **P0**: Fix FEAT0601 conflict in features.md
2. **P0**: Add FEAT0605-FEAT0620 definitions
3. **P1**: Update WebUI architecture docs with FEAT refs
4. **P1**: Add hook documentation
5. **P2**: Add store→feature mapping table
6. **P2**: Verify business rules (BR06XX)

---

## Next: Decide Phase

Formulate specific action plan with file edits.
