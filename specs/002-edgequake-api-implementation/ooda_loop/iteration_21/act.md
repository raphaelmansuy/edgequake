# Iteration 21: Swift + C# SDK Fix

## Observe
- Swift SDK: Missing ConversationService, FolderService, and related types
- C# SDK: Missing ConversationService, FolderService, DeleteNoContentAsync, and related types
- Both SDKs had conversation/folder E2E tests using XCTSkip / Skip-if-no-env

## Act
### Swift SDK
- `sdks/swift/Sources/EdgeQuakeSDK/Models.swift`: Added ConversationInfo, ConversationListResponse, ConversationDetail, ConversationMessage, CreateConversationRequest, BulkDeleteResponse, FolderInfo, CreateFolderRequest
- `sdks/swift/Sources/EdgeQuakeSDK/Services.swift`: Added ConversationService (list with wrapper unwrap, create, get, delete via deleteRaw), FolderService (list, create, delete via deleteRaw)
- `sdks/swift/Sources/EdgeQuakeSDK/EdgeQuakeClient.swift`: Registered conversations and folders
- `sdks/swift/Tests/EdgeQuakeSDKTests/E2ETest.swift`: Default tenant/user IDs, full CRUD tests

### C# SDK
- `sdks/csharp/src/EdgeQuakeSDK/Models.cs`: Added ConversationInfo, ConversationListResponse, ConversationDetail, ConversationMessage, BulkDeleteResponse, FolderInfo
- `sdks/csharp/src/EdgeQuakeSDK/HttpHelper.cs`: Added DeleteNoContentAsync()
- `sdks/csharp/src/EdgeQuakeSDK/Services.cs`: Added ConversationService, FolderService
- `sdks/csharp/src/EdgeQuakeSDK/EdgeQuakeClient.cs`: Registered conversations and folders
- `sdks/csharp/tests/EdgeQuakeSDK.Tests/E2ETest.cs`: Default tenant/user IDs, full CRUD tests

## Results
- Swift: 49 unit + 21 E2E passed, 0 skipped ✅
- C#: 50 unit + 21 E2E passed, 0 skipped ✅
