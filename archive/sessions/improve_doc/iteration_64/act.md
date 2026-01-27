# OODA Loop Iteration 64 - ACT Phase

**Date**: 2025-01-03  
**Focus**: Execute Feature Documentation Updates  
**Status**: IN PROGRESS

---

## 🎯 Objective

Update `docs/features.md` to include all 96+ missing features discovered in code scan, resolve 7 FEAT ID collisions, and establish proper namespace allocation.

---

## 📝 Changes Being Made

### 1. Add Feature ID Allocation Table

Adding to features.md header (after Quick Reference Index):

```markdown
## Feature ID Range Allocation

| Range    | Module                    | Team     | Status | Notes                                     |
| -------- | ------------------------- | -------- | ------ | ----------------------------------------- |
| FEAT00XX | Core Pipeline             | Backend  | Active | Document ingestion, chunking, embedding   |
| FEAT01XX | Query Engine              | Backend  | Active | Multi-mode query, streaming, response gen |
| FEAT02XX | Graph Operations          | Backend  | Active | Entity storage, graph queries             |
| FEAT03XX | Streaming & Pipeline      | Backend  | Active | SSE streaming, chain-of-thought           |
| FEAT04XX | Conversations & Citations | Frontend | Active | Chat UI, source citations                 |
| FEAT05XX | PDF Extraction            | Backend  | Active | Basic & SOTA PDF text extraction          |
| FEAT06XX | WebUI Infrastructure      | Frontend | Active | Core UI, layouts, progress, state         |
| FEAT07XX | WebUI API & Utils         | Frontend | Active | API client, i18n, storage, WebSocket      |
| FEAT08XX | Authentication            | Backend  | Active | API keys, JWT, RBAC (backend auth)        |
| FEAT085X | Cost Management           | Frontend | Active | WebUI cost tracking & visualization       |
| FEAT086X | WebUI Providers           | Frontend | Active | React context providers, composition      |
| FEAT10XX | Document Management UI    | Frontend | Active | Document detail views, components         |

**Allocation Rules**:

1. Backend team: FEAT00XX-05XX, FEAT08XX
2. Frontend team: FEAT06XX-07XX, FEAT085X-086X, FEAT10XX
3. New range requests: coordinate with team lead, document in features.md
4. All @implements must be added to features.md in same PR
5. Run `scripts/validate_feat_ids.sh` before commit
```

---

### 2. Resolve 7 FEAT ID Collisions

#### Collision Group 1: FEAT0636-0640 (Empty State vs Various)

**Before**: Multiple features sharing same IDs
**After**: Split into separate unique IDs

| Old ID   | Component 1                 | New ID   | Component 2      | Reassigned To |
| -------- | --------------------------- | -------- | ---------------- | ------------- |
| FEAT0636 | Empty state pattern (keep)  | FEAT0636 | Debounce perf    | → FEAT0869    |
| FEAT0637 | Contextual messaging (keep) | FEAT0637 | Node expansion   | → FEAT0870    |
| FEAT0638 | ForceAtlas2 layout (keep)   | FEAT0638 | WS visual status | → FEAT0871    |
| FEAT0639 | Keyboard navigation (keep)  | FEAT0639 | API testing      | → FEAT0872    |
| FEAT0640 | Focus management (keep)     | FEAT0640 | Request viz      | → FEAT0873    |

**Action**: Update code files to use new IDs after docs updated

#### Collision Group 2: FEAT0801-0803 (Backend Auth vs Frontend Cost)

**Current State**:

- Backend Auth: FEAT0801-0803 (implemented, stable, in docs)
- Frontend Cost: Also using FEAT0801-0803 (active, in code)

**Resolution**: Frontend cost features move to FEAT085X range

| Feature                     | Current (Code) | New (After Update) | Module        |
| --------------------------- | -------------- | ------------------ | ------------- |
| Per-document cost tracking  | FEAT0801       | → **FEAT0850**     | Frontend Cost |
| Real-time ingestion updates | FEAT0802       | → **FEAT0851**     | Frontend Cost |
| Workspace cost summary      | FEAT0803       | → **FEAT0852**     | Frontend Cost |
| Token usage breakdown       | FEAT0804       | → **FEAT0853**     | Frontend Cost |

**Backend Auth KEEPS FEAT0801-0803** (already documented, stable)

**Action**:

1. Add FEAT085X Cost section to docs
2. Update code files: hooks/use-cost.ts, stores/use-cost-store.ts, types/cost.ts

#### Collision Group 3: FEAT0800 (Theme Support)

**Current**: FEAT0800 used for Theme in code, but range reserved for Auth in docs
**Resolution**: Theme stays at FEAT0800, no conflict since Auth is FEAT0801+

---

### 3. Add FEAT04XX - Conversations & Source Citations (7 features)

```markdown
## Conversation & Citation Features (FEAT04XX)

### FEAT0401 - Clickable Entity Citations

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0401                                                                             |
| **Name**           | Clickable Entity Citations with Hover Preview                                        |
| **Module**         | edgequake_webui                                                                      |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx) |
| **Description**    | Display entity citations as interactive elements with hover preview cards            |
| **Related**        | FEAT0007, BR0301                                                                     |

### FEAT0402 - Document Deep-Links

| Attribute          | Value                                                                                                                                                           |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0402                                                                                                                                                        |
| **Name**           | Document Deep-Links with Line Numbers                                                                                                                           |
| **Module**         | edgequake_webui                                                                                                                                                 |
| **Status**         | ✅ Stable                                                                                                                                                       |
| **Code Reference** | [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx), [use-conversations.ts](../edgequake_webui/src/hooks/use-conversations.ts) |
| **Description**    | Generate URLs to specific document locations with line number anchors                                                                                           |
| **Related**        | FEAT0401, BR0302                                                                                                                                                |

### FEAT0403 - Confidence Score Visualization

| Attribute          | Value                                                                                                                                                                      |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0403                                                                                                                                                                   |
| **Name**           | Confidence Score Visualization                                                                                                                                             |
| **Module**         | edgequake_webui                                                                                                                                                            |
| **Status**         | ✅ Stable                                                                                                                                                                  |
| **Code Reference** | [source-citations.tsx](../edgequake_webui/src/components/query/source-citations.tsx), [use-conversation-store.ts](../edgequake_webui/src/stores/use-conversation-store.ts) |
| **Description**    | Display relevance/confidence scores for query results and sources                                                                                                          |
| **Related**        | FEAT0401                                                                                                                                                                   |

### FEAT0404 - Active Conversation Tracking

| Attribute          | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| **ID**             | FEAT0404                                                                             |
| **Name**           | Active Conversation Tracking                                                         |
| **Module**         | edgequake_webui                                                                      |
| **Status**         | ✅ Stable                                                                            |
| **Code Reference** | [use-conversation-store.ts](../edgequake_webui/src/stores/use-conversation-store.ts) |
| **Description**    | Track and persist currently active conversation context                              |
| **Related**        | FEAT0403, BR0601                                                                     |
```

_(Continue adding 50+ more features...)_

---

### 4. Expand FEAT05XX - Add Lineage Features

```markdown
### FEAT0540 - Chunk Detail Retrieval

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0540                                                      |
| **Name**           | Chunk Detail Retrieval API                                    |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [use-lineage.ts](../edgequake_webui/src/hooks/use-lineage.ts) |
| **Description**    | Fetch detailed chunk content with metadata                    |
| **Related**        | FEAT0541                                                      |

### FEAT0541 - Entity Provenance Tracking

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0541                                                      |
| **Name**           | Entity Provenance and Source Tracking                         |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [use-lineage.ts](../edgequake_webui/src/hooks/use-lineage.ts) |
| **Description**    | Track entity origin through document → chunk → extraction     |
| **Related**        | FEAT0540, FEAT0701                                            |

### FEAT0583 - Folder Organization

| Attribute          | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| **ID**             | FEAT0583                                                      |
| **Name**           | Conversation Folder Organization                              |
| **Module**         | edgequake_webui                                               |
| **Status**         | ✅ Stable                                                     |
| **Code Reference** | [use-folders.ts](../edgequake_webui/src/hooks/use-folders.ts) |
| **Description**    | Organize conversations into user-defined folders              |
| **Related**        | FEAT0628, BR0603                                              |
```

---

### 5. Add FEAT085X - Cost Management (Frontend)

```markdown
## Cost Management Features (FEAT085X)

> Frontend cost tracking and visualization. Backend auth is FEAT08XX (FEAT0801-0803).

### FEAT0850 - Per-Document Cost Tracking

| Attribute          | Value                                                                                                                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **ID**             | FEAT0850                                                                                                                                                                                   |
| **Name**           | Per-Document Cost Tracking                                                                                                                                                                 |
| **Module**         | edgequake_webui                                                                                                                                                                            |
| **Status**         | ✅ Stable                                                                                                                                                                                  |
| **Code Reference** | [types/cost.ts](../edgequake_webui/src/types/cost.ts), [hooks/use-cost.ts](../edgequake_webui/src/hooks/use-cost.ts), [use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts) |
| **Description**    | Track LLM API costs per ingested document                                                                                                                                                  |
| **Related**        | FEAT0851, BR0611                                                                                                                                                                           |

### FEAT0851 - Real-Time Ingestion Cost Updates

| Attribute          | Value                                                                |
| ------------------ | -------------------------------------------------------------------- |
| **ID**             | FEAT0851                                                             |
| **Name**           | Real-Time Ingestion Cost Updates                                     |
| **Module**         | edgequake_webui                                                      |
| **Status**         | ✅ Stable                                                            |
| **Code Reference** | [use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts) |
| **Description**    | Update cost metrics in real-time during document ingestion           |
| **Related**        | FEAT0850, FEAT0602                                                   |

### FEAT0852 - Workspace Cost Summary

| Attribute          | Value                                                                                                                               |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **ID**             | FEAT0852                                                                                                                            |
| **Name**           | Workspace Cost Summary and Aggregation                                                                                              |
| **Module**         | edgequake_webui                                                                                                                     |
| **Status**         | ✅ Stable                                                                                                                           |
| **Code Reference** | [hooks/use-cost.ts](../edgequake_webui/src/hooks/use-cost.ts), [use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts) |
| **Description**    | Aggregate and display total workspace costs                                                                                         |
| **Related**        | FEAT0850, FEAT0851                                                                                                                  |

### FEAT0853 - Token Usage Breakdown

| Attribute          | Value                                                  |
| ------------------ | ------------------------------------------------------ |
| **ID**             | FEAT0853                                               |
| **Name**           | Token Usage Breakdown by Stage                         |
| **Module**         | edgequake_webui                                        |
| **Status**         | ✅ Stable                                              |
| **Code Reference** | [types/cost.ts](../edgequake_webui/src/types/cost.ts)  |
| **Description**    | Display input/output token counts per processing stage |
| **Related**        | FEAT0850, FEAT1046                                     |
```

---

## 📊 Update Summary Statistics

**Before**:

```markdown
| **TOTAL** | **104** | **101** | **2** | **1** |
```

**After** (Target):

```markdown
| Category             | Total   | Stable  | Beta  | Planned |
| -------------------- | ------- | ------- | ----- | ------- |
| Core RAG             | 20      | 20      | 0     | 0       |
| Query Engine         | 10      | 10      | 0     | 0       |
| Storage              | 5       | 5       | 0     | 0       |
| Pipeline             | 4       | 4       | 0     | 0       |
| Conversations        | 7       | 7       | 0     | 0       |
| Lineage & Context    | 6       | 6       | 0     | 0       |
| PDF (Basic)          | 5       | 5       | 0     | 0       |
| PDF (Advanced)       | 14      | 12      | 2     | 0       |
| WebUI Core           | 55      | 54      | 0     | 1       |
| WebUI API & Utils    | 37      | 37      | 0     | 0       |
| Auth (Backend)       | 3       | 3       | 0     | 0       |
| Cost Mgmt (Frontend) | 4       | 4       | 0     | 0       |
| WebUI Providers      | 10      | 10      | 0     | 0       |
| Document Mgmt UI     | 44      | 44      | 0     | 0       |
| **TOTAL**            | **224** | **221** | **2** | **1**   |
```

**Change**: +120 features documented (104 → 224)

---

## 🔄 Code Files Requiring Updates

After features.md is updated, these code files need @implements ID changes:

1. **hooks/use-debounce.ts**: FEAT0636 → FEAT0869
2. **hooks/use-graph-expansion.ts**: FEAT0637 → FEAT0870
3. **components/shared/websocket-status.tsx**: FEAT0638 → FEAT0871
4. **components/shared/api-explorer.tsx**: FEAT0639 → FEAT0872, FEAT0640 → FEAT0873
5. **hooks/use-cost.ts**: FEAT0801 → FEAT0850, FEAT0803 → FEAT0852
6. **stores/use-cost-store.ts**: FEAT0801 → FEAT0850, FEAT0802 → FEAT0851, FEAT0803 → FEAT0852
7. **types/cost.ts**: FEAT0801 → FEAT0850, FEAT0804 → FEAT0853

**Total Files**: 7  
**Total ID Changes**: 13

---

## ✅ Completion Checklist

- [ ] Update features.md version to 1.4.0
- [ ] Add Feature ID Range Allocation table
- [ ] Add FEAT04XX section (Conversations, 7 features)
- [ ] Expand FEAT05XX section (Lineage, +3 features)
- [ ] Expand FEAT06XX section (WebUI Core, +55 features)
- [ ] Expand FEAT07XX section (API & Utils, +20 features)
- [ ] Add FEAT074X section (Query Interface, +4 features)
- [ ] Add FEAT076X section (Progress, +1 feature)
- [ ] Add FEAT085X section (Cost Management, +4 features)
- [ ] Add FEAT086X section (WebUI Providers, +10 features)
- [ ] Expand FEAT10XX section (Document UI, +44 features)
- [ ] Update Quick Reference Index
- [ ] Update Summary Statistics table
- [ ] Update business_rules.md references (if needed)
- [ ] Update use_cases.md references (if needed)
- [ ] Update code files with new FEAT IDs
- [ ] Run validation: `grep -roh "@implements FEAT[0-9]{4}" edgequake_webui/src/ | sort | uniq -d` (should return empty)
- [ ] Commit changes with detailed message

---

**Status**: Ready to execute massive features.md update...
