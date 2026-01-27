# Decide - Iteration 58

## Action Plan

### P0 Actions (Execute Immediately)

#### 1. Redefine WebUI Features (FEAT06XX)

**Target File**: `docs/features.md`

**Changes**:

1. Rename existing FEAT0601-0604 to match code references
2. Add FEAT0605-FEAT0620 with proper code references
3. Update feature count in summary table
4. Fix cross-references

**New Feature Definitions**:

```markdown
## WebUI Features (FEAT06XX)

### FEAT0601 - Knowledge Graph Visualization

- Module: edgequake_webui
- Store: use-graph-store.ts (950 lines)
- Implements: Interactive graph with Sigma.js, node selection, zoom/pan

### FEAT0602 - Chat Query Interface

- Module: edgequake_webui
- Store: use-query-store.ts
- Implements: Conversational RAG interface

### FEAT0603 - Streaming Response Display

- Module: edgequake_webui
- Hook: use-graph-stream.ts
- Implements: SSE streaming with progressive rendering

### FEAT0604 - Query Mode Selector

- Module: edgequake_webui
- Store: use-query-ui-store.ts
- Implements: hybrid/local/global/naive mode selection

### FEAT0605 - Document Upload Interface

- Module: edgequake_webui
- Store: use-ingestion-store.ts
- Implements: Drag-drop file upload with progress

### FEAT0606 - Workspace Switcher

- Module: edgequake_webui
- Store: use-tenant-store.ts
- Implements: Multi-workspace navigation

### FEAT0607 - Entity Type Filter

- Module: edgequake_webui
- Store: use-graph-store.ts
- Implements: Filter nodes by entity type

### FEAT0608 - Graph Bookmark Manager

- Module: edgequake_webui
- Store: use-graph-store.ts
- Implements: Save/load graph views

### FEAT0609 - Conversation Persistence

- Module: edgequake_webui
- Store: use-conversation-store.ts
- Implements: localStorage + backend sync

### FEAT0610 - Cost Tracking Display

- Module: edgequake_webui
- Store: use-cost-store.ts
- Implements: Token usage and cost estimates

### FEAT0611 - Ingestion Progress Monitor

- Module: edgequake_webui
- Hook: use-ingestion-progress.ts
- Implements: Real-time ingestion status

### FEAT0612 - Keyboard Navigation

- Module: edgequake_webui
- Hook: use-keyboard-shortcuts.ts
- Implements: Graph keyboard controls

### FEAT0613 - Dark/Light Theme

- Module: edgequake_webui
- Store: use-settings-store.ts
- Implements: Theme toggle with persistence

### FEAT0614 - Multi-Language (i18n)

- Module: edgequake_webui
- Lib: i18next + react-i18next
- Implements: Internationalization support

### FEAT0615 - Source Citation Links

- Module: edgequake_webui
- Hook: use-lineage.ts
- Implements: Deep links to source documents

### FEAT0616 - Entity Search (MiniSearch)

- Module: edgequake_webui
- Store: use-graph-store.ts
- Implements: Client-side entity search

### FEAT0617 - User Preference Persistence

- Module: edgequake_webui
- Store: use-settings-store.ts
- Implements: localStorage persistence

### FEAT0618 - Graph Layout Settings

- Module: edgequake_webui
- Store: use-settings-store.ts
- Implements: Force/circular layout config

### FEAT0619 - Ingestion Quality Settings

- Module: edgequake_webui
- Store: use-settings-store.ts
- Implements: Gleaning, summarization config

### FEAT0620 - Query Result Export

- Module: edgequake_webui
- Status: 🔧 Planned
- Implements: Export to JSON/CSV
```

### P1 Actions (Next Iteration)

1. **Update 0014-webui-state-management.md**

   - Add store-feature mapping table
   - Add hook catalog

2. **Create Hooks Catalog**
   - Document all 20 hooks in a dedicated section

### P2 Actions (Future)

1. Add BR06XX business rules for WebUI
2. Add UC06XX use cases for WebUI workflows

---

## Execution Order

```
┌─────────────────────────────────────────────┐
│         Iteration 58 Execution              │
├─────────────────────────────────────────────┤
│                                             │
│  1. ────────────────────────────────────▶   │
│     Update features.md WebUI section        │
│     - Redefine FEAT0601-0604               │
│     - Add FEAT0605-FEAT0620                │
│                                             │
│  2. ────────────────────────────────────▶   │
│     Update quick reference index            │
│     - Change count from 4 to 20            │
│                                             │
│  3. ────────────────────────────────────▶   │
│     Update summary statistics               │
│     - WebUI: 4 → 20 features               │
│     - Total: 71 → 87 features              │
│                                             │
│  4. ────────────────────────────────────▶   │
│     Commit with OODA-58 reference          │
│                                             │
└─────────────────────────────────────────────┘
```

---

## Next: Act Phase

Execute the file modifications.
