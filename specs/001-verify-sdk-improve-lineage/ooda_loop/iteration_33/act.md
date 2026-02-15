# OODA-33 Act: PHP SDK Complete Coverage Implementation

## Changes Made

### 1. HttpHelper.php

**File**: `sdks/php/src/HttpHelper.php`

- Added `put()` method (line 26-29)
- Added `patch()` method (line 31-34)

### 2. Services.php

**File**: `sdks/php/src/Services.php`

#### New Service Classes

- `AuthService` (lines 230-260): login, logout, refresh, me
- `WorkspaceService` (lines 262-310): list, get, create, update, delete, metricsHistory, rebuildEmbeddings, rebuildKnowledgeGraph, reprocessDocuments
- `SharedService` (lines 312-320): get

#### Enhanced Services

- `HealthService`: +3 methods (ready, live, metrics)
- `DocumentService`: +5 methods (deleteAll, reprocess, recoverStuck, retryChunks, failedChunks)
- `EntityService`: +3 methods (update, merge, neighborhood)
- `RelationshipService`: +3 methods (get, update, delete)
- `GraphService`: +4 methods (getNode, searchLabels, popularLabels, degreesBatch)
- `TenantService`: +4 methods (get, create, update, delete)
- `UserService`: +3 methods (get, create, delete)
- `ApiKeyService`: +3 methods (create, delete, revoke)
- `TaskService`: +3 methods (get, cancel, retry)
- `PipelineService`: +1 method (cancel)
- `ModelService`: +5 methods (listLlm, listEmbedding, getProvider, getModel, listProviders)
- `CostService`: +4 methods (history, pricing, estimate, updateBudget)
- `ConversationService`: +10 methods (get, update, delete, import, share, unshare, listMessages, createMessage, bulkArchive, bulkMove)
- `FolderService`: +3 methods (get, update, delete)

### 3. Client.php

**File**: `sdks/php/src/Client.php`

- Added AuthService property and initialization
- Added WorkspaceService property and initialization
- Added SharedService property and initialization
- Updated docblock with OODA-33 reference

### 4. MockHttpHelper.php

**File**: `sdks/php/tests/MockHttpHelper.php`

- Added `put()` method
- Added `patch()` method
- Added separate `request()` method for JSON decode

### 5. UnitTest.php

**File**: `sdks/php/tests/UnitTest.php`

- Added imports for AuthService, WorkspaceService, SharedService
- Updated testClientInitializesAllServices with new services
- Added 72 new test methods covering all new endpoints

## Test Results

```
PHPUnit 11.5.53
Tests: 178, Assertions: 338
Status: OK (with 1 warning about E2ETest)
```

## Metrics Summary

| Metric          | Before              | After          | Change     |
| --------------- | ------------------- | -------------- | ---------- |
| Service classes | 17                  | 20             | +3         |
| Public methods  | ~30                 | ~80            | +50        |
| Unit tests      | 106                 | 178            | +72 (+68%) |
| HTTP verbs      | 3 (GET/POST/DELETE) | 5 (+PUT/PATCH) | +2         |

## Commit

```bash
git add -f sdks/php/ specs/001-verify-sdk-improve-lineage/ooda_loop/iteration_33/
git commit -m "OODA-33: PHP SDK complete API coverage (+50 methods, +72 tests)"
```
