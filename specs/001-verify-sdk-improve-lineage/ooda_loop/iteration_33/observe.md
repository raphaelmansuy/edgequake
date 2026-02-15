# OODA-33 Observe: PHP SDK Coverage Audit

## Observation Date

2026-01-17

## SDK State Before

- **Services**: 17 classes
- **Methods**: ~30 public methods
- **Tests**: 106 passing (lowest of all SDKs)
- **HTTP methods**: GET, POST, DELETE (missing PUT, PATCH)

## Endpoint Gap Analysis

### Missing HTTP Methods

- `HttpHelper` lacked `put()` and `patch()` methods
- Many update/modify operations were not possible

### Missing Service Classes

| Service          | Endpoints                  | Priority |
| ---------------- | -------------------------- | -------- |
| AuthService      | login, logout, refresh, me | High     |
| WorkspaceService | CRUD + rebuild + metrics   | High     |
| SharedService    | get shared conversations   | Medium   |

### Missing Methods in Existing Services

| Service             | Missing Methods                                                                                 |
| ------------------- | ----------------------------------------------------------------------------------------------- |
| HealthService       | ready, live, metrics                                                                            |
| DocumentService     | deleteAll, reprocess, recoverStuck, retryChunks, failedChunks                                   |
| EntityService       | update, merge, neighborhood                                                                     |
| RelationshipService | get, update, delete                                                                             |
| GraphService        | getNode, searchLabels, popularLabels, degreesBatch                                              |
| TenantService       | get, create, update, delete                                                                     |
| UserService         | get, create, delete                                                                             |
| ApiKeyService       | create, delete, revoke                                                                          |
| TaskService         | get, cancel, retry                                                                              |
| PipelineService     | cancel                                                                                          |
| ModelService        | listLlm, listEmbedding, getProvider, getModel, listProviders                                    |
| CostService         | history, pricing, estimate, updateBudget                                                        |
| ConversationService | get, update, delete, import, share, unshare, listMessages, createMessage, bulkArchive, bulkMove |
| FolderService       | get, update, delete                                                                             |

### Coverage Metrics

- **Backend routes**: 135+ endpoints
- **PHP SDK coverage before**: ~22% (30/135)
- **PHP SDK coverage target**: 95%+

## MockHttpHelper Assessment

- Needed `put()` and `patch()` overrides
- Needed separate `request()` method for non-raw responses
