import Foundation

// MARK: - Service classes

/// WHY: Each service maps 1:1 to an API resource for discoverability.
/// OODA-35: Enhanced with complete API coverage (~80 methods across 20 services).

public final class HealthService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func check() async throws -> HealthResponse {
        try await http.get("/health")
    }

    public func readiness() async throws -> ReadinessResponse {
        try await http.get("/health/ready")
    }

    public func liveness() async throws -> LivenessResponse {
        try await http.get("/health/live")
    }

    public func detailed() async throws -> DetailedHealthResponse {
        try await http.get("/health/detailed")
    }
}

public final class DocumentService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> ListDocumentsResponse {
        try await http.get("/api/v1/documents?page=\(page)&page_size=\(pageSize)")
    }

    public func get(id: String) async throws -> Document {
        try await http.get("/api/v1/documents/\(id)")
    }

    public func uploadText(title: String, content: String) async throws -> UploadResponse {
        try await http.post(
            "/api/v1/documents", body: TextUploadRequest(title: title, content: content))
    }

    /// Upload text with a TextUploadRequest object.
    public func uploadText(request: TextUploadRequest) async throws -> UploadResponse {
        try await http.post("/api/v1/documents", body: request)
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/documents/\(id)")
    }

    // OODA-35: New document methods
    public func update(id: String, title: String? = nil, content: String? = nil) async throws
        -> Document
    {
        var body: [String: String] = [:]
        if let t = title { body["title"] = t }
        if let c = content { body["content"] = c }
        return try await http.put("/api/v1/documents/\(id)", body: body)
    }

    public func search(query: String, page: Int = 1, pageSize: Int = 20) async throws
        -> ListDocumentsResponse
    {
        let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        return try await http.get(
            "/api/v1/documents/search?q=\(encoded)&page=\(page)&page_size=\(pageSize)")
    }

    public func chunks(id: String) async throws -> DocumentChunksResponse {
        try await http.get("/api/v1/documents/\(id)/chunks")
    }

    public func status(id: String) async throws -> DocumentStatusResponse {
        try await http.get("/api/v1/documents/\(id)/status")
    }

    public func reprocess(id: String) async throws -> UploadResponse {
        try await http.post("/api/v1/documents/\(id)/reprocess")
    }
}

public final class EntityService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> EntityListResponse {
        try await http.get("/api/v1/graph/entities?page=\(page)&page_size=\(pageSize)")
    }

    public func get(name: String) async throws -> EntityDetailResponse {
        try await http.get("/api/v1/graph/entities/\(name)")
    }

    /// Get entity by ID. WHY: Alias for get(name:) — entity names are the primary key.
    public func get(id: String) async throws -> EntityDetailResponse {
        try await get(name: id)
    }

    public func create(_ request: CreateEntityRequest) async throws -> CreateEntityResponse {
        try await http.post("/api/v1/graph/entities", body: request)
    }

    /// Create entity with explicit label.
    public func create(request: CreateEntityRequest) async throws -> CreateEntityResponse {
        try await create(request)
    }

    public func delete(name: String) async throws -> EntityDeleteResponse {
        try await http.delete("/api/v1/graph/entities/\(name)?confirm=true")
    }

    /// Delete entity by ID. WHY: Alias for delete(name:).
    public func delete(id: String) async throws -> EntityDeleteResponse {
        try await delete(name: id)
    }

    public func exists(name: String) async throws -> EntityExistsResponse {
        try await http.get("/api/v1/graph/entities/exists?entity_name=\(name)")
    }

    // OODA-35: New entity methods
    public func update(name: String, description: String? = nil, entityType: String? = nil)
        async throws -> EntityDetailResponse
    {
        var body: [String: String] = [:]
        if let d = description { body["description"] = d }
        if let t = entityType { body["entity_type"] = t }
        return try await http.put("/api/v1/graph/entities/\(name)", body: body)
    }

    public func merge(sourceName: String, targetName: String) async throws -> MergeEntitiesResponse
    {
        try await http.post(
            "/api/v1/graph/entities/merge",
            body: ["source_name": sourceName, "target_name": targetName])
    }

    public func types() async throws -> EntityTypesResponse {
        try await http.get("/api/v1/graph/entities/types")
    }
}

public final class RelationshipService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> RelationshipListResponse {
        try await http.get("/api/v1/graph/relationships?page=\(page)&page_size=\(pageSize)")
    }

    // OODA-35: New relationship methods
    public func create(
        source: String, target: String, relationshipType: String, weight: Double = 1.0
    ) async throws -> Relationship {
        try await http.post(
            "/api/v1/graph/relationships",
            body: [
                "source": source, "target": target, "relationship_type": relationshipType,
                "weight": String(weight),
            ])
    }

    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/graph/relationships/\(id)")
    }

    public func types() async throws -> RelationshipTypesResponse {
        try await http.get("/api/v1/graph/relationships/types")
    }
}

public final class GraphService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func get() async throws -> GraphResponse {
        try await http.get("/api/v1/graph")
    }

    public func search(query: String) async throws -> SearchNodesResponse {
        let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        return try await http.get("/api/v1/graph/nodes/search?q=\(encoded)")
    }

    // OODA-35: New graph methods
    public func stats() async throws -> GraphStatsResponse {
        try await http.get("/api/v1/graph/stats")
    }

    public func clear() async throws {
        _ = try await http.deleteRaw("/api/v1/graph?confirm=true")
    }

    public func neighbors(name: String, depth: Int = 1) async throws -> GraphResponse {
        let encoded = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return try await http.get("/api/v1/graph/neighbors/\(encoded)?depth=\(depth)")
    }

    public func subgraph(entityNames: [String]) async throws -> GraphResponse {
        try await http.post("/api/v1/graph/subgraph", body: ["entity_names": entityNames])
    }
}

public final class QueryService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func execute(query: String, mode: String = "hybrid") async throws -> QueryResponse {
        try await http.post("/api/v1/query", body: QueryRequest(query: query, mode: mode))
    }

    /// Execute query with a QueryRequest object.
    public func query(request: QueryRequest) async throws -> QueryResponse {
        try await http.post("/api/v1/query", body: request)
    }
}

public final class ChatService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func completions(_ request: ChatCompletionRequest) async throws -> ChatCompletionResponse
    {
        try await http.post("/api/v1/chat/completions", body: request)
    }

    /// Convenience alias for `completions`.
    public func complete(request: ChatCompletionRequest) async throws -> ChatCompletionResponse {
        try await completions(request)
    }

    /// Get a conversation by ID. WHY: Maps to GET /api/v1/conversations/{id}.
    public func getConversation(id: String) async throws -> ConversationDetail {
        try await http.get("/api/v1/conversations/\(id)")
    }

    /// List all conversations. WHY: Maps to GET /api/v1/conversations.
    public func listConversations() async throws -> ConversationListResponse {
        try await http.get("/api/v1/conversations")
    }

    /// Bulk delete conversations. WHY: Maps to POST /api/v1/conversations/bulk/delete.
    public func bulkDeleteConversations(ids: [String]) async throws -> BulkDeleteResponse {
        try await http.post("/api/v1/conversations/bulk/delete", body: ["ids": ids])
    }

    /// List conversation folders. WHY: Maps to GET /api/v1/folders.
    public func listFolders() async throws -> [FolderInfo] {
        try await http.get("/api/v1/folders")
    }
}

public final class TenantService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> TenantListResponse {
        try await http.get("/api/v1/tenants")
    }

    // OODA-35: New tenant methods
    public func get(id: String) async throws -> TenantInfo {
        try await http.get("/api/v1/tenants/\(id)")
    }

    public func create(name: String) async throws -> TenantInfo {
        try await http.post("/api/v1/tenants", body: ["name": name])
    }

    public func update(id: String, name: String) async throws -> TenantInfo {
        try await http.put("/api/v1/tenants/\(id)", body: ["name": name])
    }

    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/tenants/\(id)")
    }
}

public final class UserService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> UserListResponse {
        try await http.get("/api/v1/users")
    }

    // OODA-35: New user methods
    public func get(id: String) async throws -> UserInfo {
        try await http.get("/api/v1/users/\(id)")
    }

    public func create(email: String, name: String? = nil, role: String = "user") async throws
        -> UserInfo
    {
        var body: [String: String] = ["email": email, "role": role]
        if let n = name { body["name"] = n }
        return try await http.post("/api/v1/users", body: body)
    }

    public func update(id: String, name: String? = nil, role: String? = nil) async throws
        -> UserInfo
    {
        var body: [String: String] = [:]
        if let n = name { body["name"] = n }
        if let r = role { body["role"] = r }
        return try await http.put("/api/v1/users/\(id)", body: body)
    }

    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/users/\(id)")
    }
}

public final class ApiKeyService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> ApiKeyListResponse {
        try await http.get("/api/v1/api-keys")
    }

    // OODA-35: New API key methods
    public func get(id: String) async throws -> ApiKeyInfo {
        try await http.get("/api/v1/api-keys/\(id)")
    }

    public func create(name: String) async throws -> CreateApiKeyResponse {
        try await http.post("/api/v1/api-keys", body: ["name": name])
    }

    public func revoke(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/api-keys/\(id)")
    }

    public func rotate(id: String) async throws -> CreateApiKeyResponse {
        try await http.post("/api/v1/api-keys/\(id)/rotate")
    }
}

public final class TaskService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> TaskListResponse {
        try await http.get("/api/v1/tasks")
    }

    // OODA-35: New task methods
    public func get(id: String) async throws -> TaskInfo {
        try await http.get("/api/v1/tasks/\(id)")
    }

    public func create(taskType: String) async throws -> TaskInfo {
        try await http.post("/api/v1/tasks", body: ["task_type": taskType])
    }

    public func cancel(id: String) async throws -> TaskInfo {
        try await http.post("/api/v1/tasks/\(id)/cancel")
    }

    public func status(id: String) async throws -> TaskStatus {
        try await http.get("/api/v1/tasks/\(id)/status")
    }
}

public final class PipelineService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func status() async throws -> PipelineStatus {
        try await http.get("/api/v1/pipeline/status")
    }

    public func queueMetrics() async throws -> QueueMetrics {
        try await http.get("/api/v1/pipeline/queue-metrics")
    }

    // OODA-35: New pipeline methods
    public func processingList() async throws -> ProcessingListResponse {
        try await http.get("/api/v1/pipeline/processing")
    }

    public func pause() async throws -> PipelineStatus {
        try await http.post("/api/v1/pipeline/pause")
    }

    public func resume() async throws -> PipelineStatus {
        try await http.post("/api/v1/pipeline/resume")
    }

    public func config() async throws -> PipelineConfig {
        try await http.get("/api/v1/pipeline/config")
    }
}

public final class ModelService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func catalog() async throws -> ProviderCatalog {
        try await http.get("/api/v1/models")
    }

    public func health() async throws -> [ProviderHealthInfo] {
        let data = try await http.getRaw("/api/v1/models/health")
        return try http.decodeJSON([ProviderHealthInfo].self, from: data)
    }

    public func providerStatus() async throws -> ProviderStatus {
        try await http.get("/api/v1/settings/provider/status")
    }

    /// Get named provider health. WHY: Maps to GET /api/v1/models/health/{name}.
    public func providerHealth(name: String) async throws -> ProviderHealthInfo {
        let encoded = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return try await http.get("/api/v1/models/health/\(encoded)")
    }

    /// Alias for providerStatus(). WHY: Convenience for tests that call status().
    public func status() async throws -> ProviderStatus {
        try await providerStatus()
    }

    // OODA-35: New model methods
    public func list() async throws -> ModelListResponse {
        try await http.get("/api/v1/models/list")
    }

    public func get(id: String) async throws -> ModelInfo {
        try await http.get("/api/v1/models/\(id)")
    }

    public func providers() async throws -> [ProviderInfo] {
        try await http.get("/api/v1/models/providers")
    }

    public func setDefault(provider: String, model: String) async throws -> ModelConfig {
        try await http.post("/api/v1/models/default", body: ["provider": provider, "model": model])
    }

    public func test(provider: String, model: String) async throws -> ModelTestResult {
        try await http.post("/api/v1/models/test", body: ["provider": provider, "model": model])
    }
}

public final class CostService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func summary() async throws -> CostSummary {
        try await http.get("/api/v1/costs/summary")
    }

    // OODA-35: New cost methods
    public func daily(days: Int = 30) async throws -> [DailyCost] {
        try await http.get("/api/v1/costs/daily?days=\(days)")
    }

    public func byProvider() async throws -> [ProviderCost] {
        try await http.get("/api/v1/costs/by-provider")
    }

    public func byModel() async throws -> [ModelCost] {
        try await http.get("/api/v1/costs/by-model")
    }

    public func export(format: String = "csv") async throws -> Data {
        try await http.getRaw("/api/v1/costs/export?format=\(format)")
    }
}

public final class ConversationService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array.
    public func list() async throws -> [ConversationInfo] {
        let wrapper: ConversationListResponse = try await http.get("/api/v1/conversations")
        return wrapper.items ?? []
    }

    public func create(title: String) async throws -> ConversationInfo {
        try await http.post("/api/v1/conversations", body: CreateConversationRequest(title: title))
    }

    public func get(id: String) async throws -> ConversationDetail {
        try await http.get("/api/v1/conversations/\(id)")
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/conversations/\(id)")
    }

    public func bulkDelete(ids: [String]) async throws -> BulkDeleteResponse {
        try await http.post("/api/v1/conversations/bulk/delete", body: ["ids": ids])
    }

    // OODA-35: New conversation methods
    public func update(id: String, title: String) async throws -> ConversationInfo {
        try await http.put("/api/v1/conversations/\(id)", body: ["title": title])
    }

    public func messages(id: String) async throws -> [MessageInfo] {
        try await http.get("/api/v1/conversations/\(id)/messages")
    }

    public func addMessage(conversationId: String, role: String, content: String) async throws
        -> MessageInfo
    {
        try await http.post(
            "/api/v1/conversations/\(conversationId)/messages",
            body: ["role": role, "content": content])
    }

    public func deleteMessage(conversationId: String, messageId: String) async throws {
        _ = try await http.deleteRaw(
            "/api/v1/conversations/\(conversationId)/messages/\(messageId)")
    }

    public func search(query: String, limit: Int = 10) async throws -> [ConversationInfo] {
        try await http.get("/api/v1/conversations/search?q=\(query)&limit=\(limit)")
    }

    public func exportMessages(id: String, format: String = "json") async throws -> Data {
        try await http.getRaw("/api/v1/conversations/\(id)/export?format=\(format)")
    }
}

public final class FolderService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> [FolderInfo] {
        try await http.get("/api/v1/folders")
    }

    public func create(name: String) async throws -> FolderInfo {
        try await http.post("/api/v1/folders", body: CreateFolderRequest(name: name))
    }

    /// WHY: DELETE returns 204 No Content — use deleteRaw to avoid decoding empty body.
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/folders/\(id)")
    }

    // OODA-35: New folder methods
    public func get(id: String) async throws -> FolderInfo {
        try await http.get("/api/v1/folders/\(id)")
    }

    public func update(id: String, name: String) async throws -> FolderInfo {
        try await http.put("/api/v1/folders/\(id)", body: ["name": name])
    }

    public func moveConversation(conversationId: String, folderId: String) async throws
        -> ConversationInfo
    {
        try await http.post(
            "/api/v1/folders/move",
            body: ["conversation_id": conversationId, "folder_id": folderId])
    }

    public func conversations(id: String) async throws -> [ConversationInfo] {
        try await http.get("/api/v1/folders/\(id)/conversations")
    }
}

// MARK: - OODA-35: New Services

public final class AuthService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// Login with credentials
    public func login(email: String, password: String) async throws -> AuthTokenResponse {
        try await http.post("/api/v1/auth/login", body: ["email": email, "password": password])
    }

    /// Logout current session
    public func logout() async throws {
        _ = try await http.postRaw("/api/v1/auth/logout")
    }

    /// Refresh access token
    public func refresh(refreshToken: String) async throws -> AuthTokenResponse {
        try await http.post("/api/v1/auth/refresh", body: ["refresh_token": refreshToken])
    }

    /// Get current user info
    public func me() async throws -> AuthUserResponse {
        try await http.get("/api/v1/auth/me")
    }

    /// Change password
    public func changePassword(currentPassword: String, newPassword: String) async throws {
        _ = try await http.postRaw(
            "/api/v1/auth/change-password",
            body: ["current_password": currentPassword, "new_password": newPassword])
    }
}

public final class WorkspaceService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// List all workspaces
    public func list() async throws -> WorkspaceListResponse {
        try await http.get("/api/v1/workspaces")
    }

    /// Get a workspace by ID
    public func get(id: String) async throws -> WorkspaceInfo {
        try await http.get("/api/v1/workspaces/\(id)")
    }

    /// Create a new workspace
    public func create(name: String) async throws -> WorkspaceInfo {
        try await http.post("/api/v1/workspaces", body: ["name": name])
    }

    /// Update a workspace
    public func update(id: String, name: String) async throws -> WorkspaceInfo {
        try await http.put("/api/v1/workspaces/\(id)", body: ["name": name])
    }

    /// Delete a workspace
    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/workspaces/\(id)")
    }

    /// Get workspace statistics
    public func stats(id: String) async throws -> WorkspaceStatsResponse {
        try await http.get("/api/v1/workspaces/\(id)/stats")
    }

    /// Switch to a workspace
    public func switchTo(id: String) async throws -> WorkspaceInfo {
        try await http.post("/api/v1/workspaces/\(id)/switch")
    }
}

public final class SharedService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// Create a shared link for a resource
    public func createLink(resourceType: String, resourceId: String) async throws
        -> SharedLinkResponse
    {
        try await http.post(
            "/api/v1/shared/links",
            body: ["resource_type": resourceType, "resource_id": resourceId])
    }

    /// Get shared link by ID
    public func getLink(id: String) async throws -> SharedLinkResponse {
        try await http.get("/api/v1/shared/links/\(id)")
    }

    /// Delete a shared link
    public func deleteLink(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/shared/links/\(id)")
    }

    /// Access shared content by token
    public func access(token: String) async throws -> SharedAccessResponse {
        try await http.get("/api/v1/shared/access/\(token)")
    }

    /// List all shared links for current user
    public func listLinks() async throws -> [SharedLinkResponse] {
        try await http.get("/api/v1/shared/links")
    }
}
