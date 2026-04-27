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

    /// WHY: Server returns plain text (`OK`), not JSON.
    public func readiness() async throws -> String {
        String(decoding: try await http.getRaw("/ready"), as: UTF8.self)
    }

    /// WHY: Server returns plain text (`OK`), not JSON.
    public func liveness() async throws -> String {
        String(decoding: try await http.getRaw("/live"), as: UTF8.self)
    }

    /// WHY: Prometheus exposition format (text), not JSON.
    public func metrics() async throws -> String {
        String(decoding: try await http.getRaw("/metrics"), as: UTF8.self)
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

    public func track(trackId: String) async throws -> TrackStatusResponse {
        let enc =
            trackId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? trackId
        return try await http.get("/api/v1/documents/track/\(enc)")
    }

    public func reprocessFailed() async throws -> SimpleStatusResponse {
        try await http.post("/api/v1/documents/reprocess", body: EmptyJsonBody())
    }

    public func recoverStuck() async throws -> SimpleStatusResponse {
        try await http.post("/api/v1/documents/recover-stuck", body: EmptyJsonBody())
    }
}

/// Encodable empty JSON object for POST bodies that require `{}`.
private struct EmptyJsonBody: Encodable {}

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

    public func neighborhood(name: String, depth: Int = 1) async throws
        -> EntityNeighborhoodResponse
    {
        let enc = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return try await http.get(
            "/api/v1/graph/entities/\(enc)/neighborhood?depth=\(depth)")
    }
}

public final class RelationshipService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(page: Int = 1, pageSize: Int = 20) async throws -> RelationshipListResponse {
        try await http.get("/api/v1/graph/relationships?page=\(page)&page_size=\(pageSize)")
    }

    public func create(
        srcId: String,
        tgtId: String,
        keywords: String,
        description: String,
        sourceId: String = "manual_entry",
        weight: Double = 0.8
    ) async throws -> CreateRelationshipResponse {
        try await http.post(
            "/api/v1/graph/relationships",
            body: CreateRelationshipBody(
                src_id: srcId,
                tgt_id: tgtId,
                keywords: keywords,
                description: description,
                source_id: sourceId,
                weight: weight
            ))
    }

    public func get(id: String) async throws -> GetRelationshipResponse {
        let enc = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/graph/relationships/\(enc)")
    }

    public func delete(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/graph/relationships/\(id)")
    }
}

private struct CreateRelationshipBody: Encodable {
    let src_id: String
    let tgt_id: String
    let keywords: String
    let description: String
    let source_id: String
    let weight: Double
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

    public func getNode(nodeId: String) async throws -> GraphNode {
        let enc = nodeId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? nodeId
        return try await http.get("/api/v1/graph/nodes/\(enc)")
    }

    public func labelSearch(query: String, limit: Int = 20) async throws -> SearchLabelsResponse {
        let q = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        return try await http.get("/api/v1/graph/labels/search?q=\(q)&limit=\(limit)")
    }

    public func popularLabels(limit: Int = 20) async throws -> PopularLabelsResponse {
        try await http.get("/api/v1/graph/labels/popular?limit=\(limit)")
    }

    public func degreesBatch(nodeIds: [String]) async throws -> BatchDegreeResponse {
        try await http.post("/api/v1/graph/degrees/batch", body: BatchDegreeRequestBody(node_ids: nodeIds))
    }
}

private struct BatchDegreeRequestBody: Encodable {
    let node_ids: [String]
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
        try await http.post(
            "/api/v1/conversations/bulk/delete", body: ["conversation_ids": ids])
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

    public func get(id: String) async throws -> UserInfo {
        try await http.get("/api/v1/users/\(id)")
    }

    public func create(
        username: String, email: String, password: String, role: String = "user"
    ) async throws -> UserInfo {
        try await http.post(
            "/api/v1/users",
            body: [
                "username": username, "email": email, "password": password, "role": role,
            ])
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

    public func create(name: String) async throws -> CreateApiKeyResponse {
        try await http.post("/api/v1/api-keys", body: ["name": name])
    }

    public func revoke(id: String) async throws {
        _ = try await http.deleteRaw("/api/v1/api-keys/\(id)")
    }
}

public final class TaskService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list() async throws -> TaskListResponse {
        try await http.get("/api/v1/tasks")
    }

    public func get(trackId: String) async throws -> TaskInfo {
        let enc = trackId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? trackId
        return try await http.get("/api/v1/tasks/\(enc)")
    }

    public func cancel(trackId: String) async throws -> TaskInfo {
        let enc = trackId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? trackId
        return try await http.post("/api/v1/tasks/\(enc)/cancel", body: EmptyJsonBody())
    }

    public func retry(trackId: String) async throws -> TaskInfo {
        let enc = trackId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? trackId
        return try await http.post("/api/v1/tasks/\(enc)/retry", body: EmptyJsonBody())
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

    public func cancel() async throws -> SimpleStatusResponse {
        try await http.post("/api/v1/pipeline/cancel", body: EmptyJsonBody())
    }
}

public final class ModelService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func catalog() async throws -> ModelsListResponse {
        try await http.get("/api/v1/models")
    }

    public func listLlmModels() async throws -> LlmModelsResponse {
        try await http.get("/api/v1/models/llm")
    }

    public func listEmbeddingModels() async throws -> EmbeddingModelsResponse {
        try await http.get("/api/v1/models/embedding")
    }

    public func health() async throws -> [ProviderHealthInfo] {
        let data = try await http.getRaw("/api/v1/models/health")
        return try http.decodeJSON([ProviderHealthInfo].self, from: data)
    }

    public func providerStatus() async throws -> ProviderStatus {
        try await http.get("/api/v1/settings/provider/status")
    }

    public func listAvailableProviders() async throws -> [ProviderInfo] {
        try await http.get("/api/v1/settings/providers")
    }

    public func getProvider(name: String) async throws -> ProviderInfo {
        let enc = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return try await http.get("/api/v1/models/\(enc)")
    }

    public func getModel(provider: String, model: String) async throws -> ModelInfo {
        let p = provider.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? provider
        let m = model.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? model
        return try await http.get("/api/v1/models/\(p)/\(m)")
    }

    /// Alias for provider status from settings.
    public func status() async throws -> ProviderStatus {
        try await providerStatus()
    }
}

public final class CostService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func summary() async throws -> WorkspaceCostSummaryResponse {
        try await http.get("/api/v1/costs/summary")
    }

    public func history(
        startDate: String? = nil,
        endDate: String? = nil,
        granularity: String? = nil
    ) async throws -> [CostHistoryPoint] {
        var parts: [String] = []
        if let s = startDate {
            let enc = s.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? s
            parts.append("start_date=\(enc)")
        }
        if let e = endDate {
            let enc = e.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? e
            parts.append("end_date=\(enc)")
        }
        if let g = granularity {
            let enc = g.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? g
            parts.append("granularity=\(enc)")
        }
        let qs = parts.isEmpty ? "" : "?\(parts.joined(separator: "&"))"
        return try await http.get("/api/v1/costs/history\(qs)")
    }

    public func budget() async throws -> BudgetInfo {
        try await http.get("/api/v1/costs/budget")
    }

    public func updateBudget(_ budget: BudgetInfo) async throws -> BudgetInfo {
        try await http.patch("/api/v1/costs/budget", body: budget)
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
        try await http.post(
            "/api/v1/conversations/bulk/delete", body: ["conversation_ids": ids])
    }

    public func update(id: String, title: String) async throws -> ConversationInfo {
        try await http.patch("/api/v1/conversations/\(id)", body: ["title": title])
    }

    public func messages(id: String) async throws -> PaginatedMessagesResponse {
        try await http.get("/api/v1/conversations/\(id)/messages")
    }

    public func addMessage(conversationId: String, role: String, content: String) async throws
        -> ConversationMessage
    {
        try await http.post(
            "/api/v1/conversations/\(conversationId)/messages",
            body: ["role": role, "content": content])
    }

    public func deleteMessage(messageId: String) async throws {
        _ = try await http.deleteRaw("/api/v1/messages/\(messageId)")
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

    public func update(id: String, name: String) async throws -> FolderInfo {
        try await http.patch("/api/v1/folders/\(id)", body: ["name": name])
    }
}

// MARK: - OODA-35: New Services

public final class AuthService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func login(username: String, password: String) async throws -> AuthTokenResponse {
        try await http.post("/api/v1/auth/login", body: ["username": username, "password": password])
    }

    public func logout() async throws {
        _ = try await http.postRaw("/api/v1/auth/logout")
    }

    public func refresh(refreshToken: String) async throws -> AuthTokenResponse {
        try await http.post("/api/v1/auth/refresh", body: ["refresh_token": refreshToken])
    }

    public func me() async throws -> AuthUserResponse {
        try await http.get("/api/v1/auth/me")
    }
}

public final class WorkspaceService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    public func list(tenantId: String) async throws -> WorkspaceListResponse {
        let enc = tenantId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? tenantId
        return try await http.get("/api/v1/tenants/\(enc)/workspaces")
    }

    public func create(tenantId: String, name: String) async throws -> WorkspaceInfo {
        let enc = tenantId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? tenantId
        return try await http.post("/api/v1/tenants/\(enc)/workspaces", body: ["name": name])
    }

    public func get(id: String) async throws -> WorkspaceInfo {
        let enc = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/workspaces/\(enc)")
    }

    public func update(id: String, name: String) async throws -> WorkspaceInfo {
        let enc = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.put("/api/v1/workspaces/\(enc)", body: ["name": name])
    }

    public func delete(id: String) async throws {
        let enc = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        _ = try await http.deleteRaw("/api/v1/workspaces/\(enc)")
    }

    public func stats(id: String) async throws -> WorkspaceStatsResponse {
        let enc = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await http.get("/api/v1/workspaces/\(enc)/stats")
    }
}

public final class SharedService: @unchecked Sendable {
    private let http: HttpHelper
    init(_ http: HttpHelper) { self.http = http }

    /// WHY: Public read — `GET /api/v1/shared/{share_id}`.
    public func get(shareId: String) async throws -> ConversationDetail {
        let enc = shareId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? shareId
        return try await http.get("/api/v1/shared/\(enc)")
    }
}
