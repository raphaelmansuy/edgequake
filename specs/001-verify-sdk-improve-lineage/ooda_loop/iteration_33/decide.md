# OODA-33 Decide: PHP SDK Complete Coverage

## Decision
Implement comprehensive API coverage for PHP SDK in single iteration.

## Implementation Plan

### 1. HttpHelper Enhancements
- [x] Add `put()` method
- [x] Add `patch()` method

### 2. New Service Classes
- [x] AuthService (4 methods: login, logout, refresh, me)
- [x] WorkspaceService (9 methods: list, get, create, update, delete, metricsHistory, rebuildEmbeddings, rebuildKnowledgeGraph, reprocessDocuments)
- [x] SharedService (1 method: get)

### 3. Enhanced Existing Services

| Service | New Methods |
|---------|------------|
| HealthService | ready, live, metrics |
| DocumentService | deleteAll, reprocess, recoverStuck, retryChunks, failedChunks |
| EntityService | update, merge, neighborhood |
| RelationshipService | get, update, delete |
| GraphService | getNode, searchLabels, popularLabels, degreesBatch |
| TenantService | get, create, update, delete |
| UserService | get, create, delete |
| ApiKeyService | create, delete, revoke |
| TaskService | get, cancel, retry |
| PipelineService | cancel |
| ModelService | listLlm, listEmbedding, getProvider, getModel, listProviders |
| CostService | history, pricing, estimate, updateBudget |
| ConversationService | get, update, delete, import, share, unshare, listMessages, createMessage, bulkArchive, bulkMove |
| FolderService | get, update, delete |

### 4. Client.php Updates
- [x] Register AuthService
- [x] Register WorkspaceService
- [x] Register SharedService

### 5. Test Coverage
- [x] Add 72+ new tests for all new methods
- [x] Update MockHttpHelper with put/patch
- [x] Verify 178 tests pass

## Expected Outcome
- **Services**: 17 → 20 (+3)
- **Methods**: ~30 → ~80 (+50)
- **Tests**: 106 → 178 (+72)
- **Coverage**: ~22% → ~60%
