# Iteration 61 - ACT Phase

## Changes Made

### 1. business_rules.md Updates

**Version**: 1.0.0 → 1.1.0

**Added BR06XX Section (12 rules)**:

| ID     | Rule                               | Validation File               |
| ------ | ---------------------------------- | ----------------------------- |
| BR0601 | Theme Persistence                  | `use-ui-preferences-store.ts` |
| BR0602 | Conversation History Persistence   | `use-conversation-store.ts`   |
| BR0603 | Graph Node Display Limits          | `use-graph-store.ts`          |
| BR0604 | Streaming State Transitions        | `use-query-store.ts`          |
| BR0605 | Keyboard Navigation                | `components/`                 |
| BR0606 | Document Upload Size Limit         | `use-ingestion-store.ts`      |
| BR0607 | API Error Display                  | `use-query-store.ts`          |
| BR0608 | Settings Validation                | `use-settings-store.ts`       |
| BR0609 | Real-time Sync Conflict Resolution | `use-backend-store.ts`        |
| BR0610 | Modal Focus Trap                   | `use-ui-preferences-store.ts` |
| BR0611 | Query History Limit                | `use-conversation-store.ts`   |
| BR0612 | Loading State Feedback             | TanStack Query hooks          |

### 2. use_cases.md Updates

**Version**: 1.0.0 → 1.1.0

**Added UC06XX Section (10 use cases)**:

| ID     | Use Case                    | Component Reference         |
| ------ | --------------------------- | --------------------------- |
| UC0601 | Visualize Knowledge Graph   | `components/graph/`         |
| UC0602 | Execute RAG Query           | `components/chat/`          |
| UC0603 | Upload Document via UI      | `components/documents/`     |
| UC0604 | Manage Conversation History | `components/conversations/` |
| UC0605 | Switch Theme                | `components/theme/`         |
| UC0606 | Configure Settings          | `components/settings/`      |
| UC0607 | Navigate with Keyboard      | All components              |
| UC0608 | View API Errors             | `components/error/`         |
| UC0609 | Expand Graph Node           | `components/graph/`         |
| UC0610 | Search Documents            | `components/documents/`     |

**Summary Statistics Updated**:

- Total use cases: 38 → 48
- WebUI Interactions: 10 new

### 3. File Reference Fixes

Fixed store references to use actual kebab-case filenames:

| Before                    | After                         |
| ------------------------- | ----------------------------- |
| `useThemeStore.ts`        | `use-ui-preferences-store.ts` |
| `useConversationStore.ts` | `use-conversation-store.ts`   |
| `useGraphStore.ts`        | `use-graph-store.ts`          |
| `useStreamingStore.ts`    | `use-query-store.ts`          |
| `useDocumentStore.ts`     | `use-ingestion-store.ts`      |
| `useSyncStore.ts`         | `use-backend-store.ts`        |

## Validation

- [x] All BR06XX rules added with correct store references
- [x] All UC06XX cases added with component paths
- [x] Quick reference index updated
- [x] Summary statistics updated
- [x] Version numbers incremented

## Files Modified

1. `docs/business_rules.md` - Added BR06XX section, fixed references
2. `docs/use_cases.md` - Added UC06XX section, updated stats

## Next Iteration Focus

- Verify component paths in UC06XX match actual structure
- Add PDF-specific use cases (UC10XX) if missing
- Cross-verify FEAT↔BR↔UC traceability chains
