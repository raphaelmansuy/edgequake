import Foundation

// MARK: - Health

public struct HealthResponse: Codable, Sendable {
    public let status: String?
    public let version: String?
    public let storageMode: String?
    public let workspaceId: String?
    public let components: [String: AnyCodable]?
    public let llmProviderName: String?
}

// OODA-35: Additional health response types
public struct ReadinessResponse: Codable, Sendable {
    public let ready: Bool?
    public let status: String?
    public let checks: [String: AnyCodable]?
}

public struct LivenessResponse: Codable, Sendable {
    public let alive: Bool?
    public let status: String?
}

public struct DetailedHealthResponse: Codable, Sendable {
    public let status: String?
    public let version: String?
    public let uptime: Double?
    public let memoryUsage: Int?
    public let cpuUsage: Double?
    public let components: [String: AnyCodable]?
    public let database: [String: AnyCodable]?
    public let llm: [String: AnyCodable]?
}

// MARK: - Documents

public struct Document: Codable, Sendable {
    public let id: String?
    public let title: String?
    public let status: String?
    public let fileType: String?
    public let createdAt: String?
    public let updatedAt: String?
    public let fileSize: Int?
    public let chunkCount: Int?
}

public struct UploadResponse: Codable, Sendable {
    public let documentId: String?
    public let status: String?
    public let message: String?
    public let trackId: String?
    public let duplicateOf: String?
}

public struct TrackStatusResponse: Codable, Sendable {
    public let trackId: String?
    public let status: String?
    public let progress: Double?
    public let message: String?
    public let documentId: String?
}

public struct SimpleStatusResponse: Codable, Sendable {
    public let status: String?
    public let message: String?
}

public struct ListDocumentsResponse: Codable, Sendable {
    public let documents: [Document]?
    public let items: [Document]?
    public let total: Int?
    public let page: Int?
    public let pageSize: Int?
    public let totalPages: Int?
    public let hasMore: Bool?
}

public struct TextUploadRequest: Codable, Sendable {
    public let title: String
    public let content: String
    public let fileType: String

    public init(title: String, content: String, fileType: String = "txt") {
        self.title = title
        self.content = content
        self.fileType = fileType
    }
}

// MARK: - Entities

public struct Entity: Codable, Sendable {
    public let id: String?
    public let entityName: String?
    public let entityType: String?
    public let description: String?
    public let sourceId: String?
    public let createdAt: String?
    public let updatedAt: String?
    public let degree: Int?
    public let metadata: [String: AnyCodable]?
}

public struct CreateEntityRequest: Codable, Sendable {
    public let entityName: String
    public let entityType: String
    public let description: String
    public let sourceId: String

    public init(entityName: String, entityType: String, description: String, sourceId: String) {
        self.entityName = entityName
        self.entityType = entityType
        self.description = description
        self.sourceId = sourceId
    }
}

public struct CreateEntityResponse: Codable, Sendable {
    public let status: String?
    public let message: String?
    public let entity: Entity?
}

public struct EntityDetailResponse: Codable, Sendable {
    public let entity: Entity?
    public let relationships: [String: AnyCodable]?
    public let statistics: [String: AnyCodable]?
}

public struct EntityListResponse: Codable, Sendable {
    public let items: [Entity]?
    public let total: Int?
    public let page: Int?
    public let pageSize: Int?
    public let totalPages: Int?
}

public struct EntityExistsResponse: Codable, Sendable {
    public let entityId: String?
    public let exists: Bool?
}

public struct EntityDeleteResponse: Codable, Sendable {
    public let status: String?
    public let message: String?
    public let deletedEntityId: String?
    public let deletedRelationships: Int?
    public let affectedEntities: [String]?
}

public struct MergeEntitiesRequest: Codable, Sendable {
    public let sourceEntity: String
    public let targetEntity: String
}

// MARK: - Relationships

public struct Relationship: Codable, Sendable {
    public let id: String?
    public let srcId: String?
    public let tgtId: String?
    public let relationType: String?
    public let keywords: String?
    public let weight: Double?
    public let description: String?
    public let sourceId: String?
    public let createdAt: String?
    public let updatedAt: String?
    public let metadata: AnyCodable?
}

public struct RelationshipListResponse: Codable, Sendable {
    public let items: [Relationship]?
    public let total: Int?
    public let page: Int?
    public let pageSize: Int?
    public let totalPages: Int?
}

public struct CreateRelationshipResponse: Codable, Sendable {
    public let status: String?
    public let message: String?
    public let relationship: Relationship?
}

public struct GetRelationshipResponse: Codable, Sendable {
    public let relationship: Relationship?
    public let entities: AnyCodable?
}

// MARK: - Graph

public struct GraphNode: Codable, Sendable {
    public let id: String?
    public let label: String?
    public let entityType: String?
    public let properties: [String: AnyCodable]?
}

public struct GraphEdge: Codable, Sendable {
    public let source: String?
    public let target: String?
    public let label: String?
    public let weight: Double?
}

public struct GraphResponse: Codable, Sendable {
    public let nodes: [GraphNode]?
    public let edges: [GraphEdge]?
}

public struct SearchNodesResponse: Codable, Sendable {
    public let nodes: [GraphNode]?
    public let total: Int?
}

public struct SearchLabelsResponse: Codable, Sendable {
    public let labels: [String]?
}

public struct PopularLabel: Codable, Sendable {
    public let label: String?
    public let entityType: String?
    public let degree: Int?
    public let description: String?
}

public struct PopularLabelsResponse: Codable, Sendable {
    public let labels: [PopularLabel]?
    public let totalEntities: Int?
}

public struct NodeDegree: Codable, Sendable {
    public let nodeId: String?
    public let degree: Int?
}

public struct BatchDegreeResponse: Codable, Sendable {
    public let degrees: [NodeDegree]?
    public let count: Int?
}

public struct NeighborhoodNode: Codable, Sendable {
    public let id: String?
    public let entityType: String?
    public let description: String?
    public let degree: Int?
}

public struct NeighborhoodEdge: Codable, Sendable {
    public let id: String?
    public let source: String?
    public let target: String?
    public let relationType: String?
    public let weight: Double?
}

public struct EntityNeighborhoodResponse: Codable, Sendable {
    public let nodes: [NeighborhoodNode]?
    public let edges: [NeighborhoodEdge]?
}

// MARK: - Query & Chat

public struct QueryRequest: Codable, Sendable {
    public let query: String
    public let mode: String
    public let systemPrompt: String?

    enum CodingKeys: String, CodingKey {
        case query, mode
        case systemPrompt = "system_prompt"
    }

    public init(query: String, mode: String = "hybrid", systemPrompt: String? = nil) {
        self.query = query
        self.mode = mode
        self.systemPrompt = systemPrompt
    }
}

public struct QueryResponse: Codable, Sendable {
    public let answer: String?
    public let sources: [AnyCodable]?
    public let mode: String?
}

public struct ChatMessage: Codable, Sendable {
    public let role: String
    public let content: String

    public init(role: String, content: String) {
        self.role = role
        self.content = content
    }
}

public struct ChatCompletionRequest: Codable, Sendable {
    public let message: String
    public let mode: String?
    public let stream: Bool
    public let systemPrompt: String?

    enum CodingKeys: String, CodingKey {
        case message, mode, stream
        case systemPrompt = "system_prompt"
    }

    public init(message: String, mode: String? = "hybrid", stream: Bool = false, systemPrompt: String? = nil) {
        self.message = message
        self.mode = mode
        self.stream = stream
        self.systemPrompt = systemPrompt
    }
}

public struct ChatCompletionResponse: Codable, Sendable {
    public let conversationId: String?
    public let userMessageId: String?
    public let assistantMessageId: String?
    public let content: String?
    public let mode: String?
    public let sources: [AnyCodable]?
    public let stats: AnyCodable?
    public let tokensUsed: Int?
    public let durationMs: Int?
    public let llmProvider: String?
    public let llmModel: String?
}

// MARK: - Auth & Multi-tenant

public struct TenantInfo: Codable, Sendable {
    public let id: String?
    public let name: String?
    public let slug: String?
    public let plan: String?
    public let isActive: Bool?
    public let maxWorkspaces: Int?
    // Default LLM configuration for new workspaces.
    public let defaultLlmModel: String?
    public let defaultLlmProvider: String?
    public let defaultLlmFullId: String?
    // Default embedding configuration for new workspaces.
    public let defaultEmbeddingModel: String?
    public let defaultEmbeddingProvider: String?
    public let defaultEmbeddingDimension: Int?
    public let defaultEmbeddingFullId: String?
    // Default vision LLM for PDF image extraction (SPEC-041).
    public let defaultVisionLlmModel: String?
    public let defaultVisionLlmProvider: String?
    public let createdAt: String?
    public let updatedAt: String?

    enum CodingKeys: String, CodingKey {
        case id, name, slug, plan
        case isActive = "is_active"
        case maxWorkspaces = "max_workspaces"
        case defaultLlmModel = "default_llm_model"
        case defaultLlmProvider = "default_llm_provider"
        case defaultLlmFullId = "default_llm_full_id"
        case defaultEmbeddingModel = "default_embedding_model"
        case defaultEmbeddingProvider = "default_embedding_provider"
        case defaultEmbeddingDimension = "default_embedding_dimension"
        case defaultEmbeddingFullId = "default_embedding_full_id"
        case defaultVisionLlmModel = "default_vision_llm_model"
        case defaultVisionLlmProvider = "default_vision_llm_provider"
        case createdAt = "created_at"
        case updatedAt = "updated_at"
    }
}

public struct TenantListResponse: Codable, Sendable {
    public let items: [TenantInfo]?
}

public struct UserInfo: Codable, Sendable {
    public let id: String?
    public let username: String?
    public let email: String?
    public let role: String?
}

public struct UserListResponse: Codable, Sendable {
    public let users: [UserInfo]?
}

public struct ApiKeyInfo: Codable, Sendable {
    public let id: String?
    public let name: String?
    public let prefix: String?
    public let createdAt: String?
}

public struct ApiKeyListResponse: Codable, Sendable {
    public let keys: [ApiKeyInfo]?
}

// MARK: - Operations

public struct PipelineStatus: Codable, Sendable {
    public let isBusy: Bool?
    public let totalDocuments: Int?
    public let processedDocuments: Int?
    public let pendingTasks: Int?
    public let processingTasks: Int?
    public let completedTasks: Int?
    public let failedTasks: Int?
    public let cancellationRequested: Bool?
}

public struct QueueMetrics: Codable, Sendable {
    public let pendingCount: Int?
    public let processingCount: Int?
    public let activeWorkers: Int?
    public let maxWorkers: Int?
    public let workerUtilization: Int?
    public let avgWaitTimeSeconds: Double?
    public let throughputPerMinute: Double?
    public let rateLimited: Bool?
}

public struct TaskInfo: Codable, Sendable {
    public let id: String?
    public let trackId: String?
    public let status: String?
    public let taskType: String?
    public let createdAt: String?
}

public struct TaskListResponse: Codable, Sendable {
    public let tasks: [TaskInfo]?
    public let items: [TaskInfo]?
    public let total: Int?
}

// MARK: - Models / Providers

public struct ProviderCatalog: Codable, Sendable {
    public let providers: [ProviderInfo]?
}

public struct ProviderInfo: Codable, Sendable {
    public let name: String?
    public let displayName: String?
    public let providerType: String?
    public let enabled: Bool?
    public let priority: Int?
    public let description: String?
    public let models: [AnyCodable]?
    public let health: AnyCodable?
}

public struct ModelsListResponse: Codable, Sendable {
    public let providers: [ProviderInfo]?
    public let defaultLlmProvider: String?
    public let defaultLlmModel: String?
    public let defaultEmbeddingProvider: String?
    public let defaultEmbeddingModel: String?
}

public struct LlmModelsResponse: Codable, Sendable {
    public let models: [AnyCodable]?
    public let defaultProvider: String?
    public let defaultModel: String?
}

public struct EmbeddingModelsResponse: Codable, Sendable {
    public let models: [AnyCodable]?
    public let defaultProvider: String?
    public let defaultModel: String?
}

public struct ProviderHealthInfo: Codable, Sendable {
    public let name: String?
    public let displayName: String?
    public let providerType: String?
    public let enabled: Bool?
    public let priority: Int?
    public let models: [AnyCodable]?
}

public struct ProviderStatus: Codable, Sendable {
    public let provider: AnyCodable?
    public let embedding: AnyCodable?
    public let storage: AnyCodable?
    public let metadata: AnyCodable?
}

// MARK: - Costs

public struct CostSummary: Codable, Sendable {
    public let totalCost: Double?
    public let documentCount: Int?
    public let queryCount: Int?
    public let entries: [AnyCodable]?
}

public struct OperationBreakdown: Codable, Sendable {
    public let operation: String?
    public let cost: Double?
    public let percentage: Double?
    public let inputTokens: Int?
    public let outputTokens: Int?
    public let totalTokens: Int?
    public let callCount: Int?
}

public struct BudgetInfo: Codable, Sendable {
    public let monthlyBudgetUsd: Double?
    public let spentUsd: Double?
    public let remainingUsd: Double?
    public let alertThreshold: Double?
    public let isOverBudget: Bool?
}

public struct WorkspaceCostSummaryResponse: Codable, Sendable {
    public let workspaceId: String?
    public let totalCost: Double?
    public let documentCount: Int?
    public let totalTokens: Int?
    public let averageCostPerDocument: Double?
    public let periodStart: String?
    public let periodEnd: String?
    public let byOperation: [OperationBreakdown]?
    public let budget: BudgetInfo?
}

public struct CostHistoryPoint: Codable, Sendable {
    public let timestamp: String?
    public let totalCost: Double?
    public let totalTokens: Int?
    public let documentCount: Int?
}

// MARK: - Conversations

public struct ConversationInfo: Codable, Sendable {
    public let id: String?
    public let tenantId: String?
    public let workspaceId: String?
    public let title: String?
    public let mode: String?
    public let isPinned: Bool?
    public let folderId: String?
    public let createdAt: String?
    public let updatedAt: String?
    public let messageCount: Int?
}

/// WHY: GET /api/v1/conversations returns {"items":[...]} wrapper, not raw array.
public struct ConversationListResponse: Codable, Sendable {
    public let items: [ConversationInfo]?
}

/// WHY: GET /api/v1/conversations/{id} returns {"conversation":{...},"messages":[...]} wrapper.
public struct ConversationDetail: Codable, Sendable {
    public let conversation: ConversationInfo?
    public let messages: [ConversationMessage]?

    /// Convenience accessor for conversation ID.
    public var id: String? { conversation?.id }
}

public struct ConversationMessage: Codable, Sendable {
    public let id: String?
    public let conversationId: String?
    public let parentId: String?
    public let role: String?
    public let content: String?
    public let mode: String?
    public let tokensUsed: Int?
    public let createdAt: String?
}

public struct PaginationMetaResponse: Codable, Sendable {
    public let nextCursor: String?
    public let prevCursor: String?
    public let total: Int?
    public let hasMore: Bool?
}

public struct PaginatedMessagesResponse: Codable, Sendable {
    public let items: [ConversationMessage]?
    public let pagination: PaginationMetaResponse?
}

public struct CreateConversationRequest: Codable, Sendable {
    public let title: String
    public let mode: String?
    public let folderId: String?

    public init(title: String, mode: String? = nil, folderId: String? = nil) {
        self.title = title
        self.mode = mode
        self.folderId = folderId
    }
}

public struct BulkDeleteResponse: Codable, Sendable {
    /// API field from `BulkOperationResponse`.
    public let affected: Int?
    /// Legacy mocks only.
    public let deleted: Int?
    public let status: String?
}

// MARK: - Folders

public struct FolderInfo: Codable, Sendable {
    public let id: String?
    public let tenantId: String?
    public let name: String?
    public let createdAt: String?
    public let updatedAt: String?
}

public struct CreateFolderRequest: Codable, Sendable {
    public let name: String

    public init(name: String) {
        self.name = name
    }
}

// MARK: - Generic JSON wrapper

/// WHY: Swift Codable doesn't support [String: Any] natively.
/// AnyCodable wraps arbitrary JSON values.
public struct AnyCodable: Codable, Sendable {
    public let value: Any

    public init(_ value: Any) { self.value = value }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            value = NSNull()
        } else if let b = try? container.decode(Bool.self) {
            value = b
        } else if let i = try? container.decode(Int.self) {
            value = i
        } else if let d = try? container.decode(Double.self) {
            value = d
        } else if let s = try? container.decode(String.self) {
            value = s
        } else if let a = try? container.decode([AnyCodable].self) {
            value = a.map { $0.value }
        } else if let d = try? container.decode([String: AnyCodable].self) {
            value = d.mapValues { $0.value }
        } else {
            value = NSNull()
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch value {
        case is NSNull: try container.encodeNil()
        case let b as Bool: try container.encode(b)
        case let i as Int: try container.encode(i)
        case let d as Double: try container.encode(d)
        case let s as String: try container.encode(s)
        case let a as [Any]: try container.encode(a.map { AnyCodable($0) })
        case let d as [String: Any]: try container.encode(d.mapValues { AnyCodable($0) })
        default: try container.encodeNil()
        }
    }
}

// MARK: - OODA-35: New Model Types for Enhanced Services

// Health Extended
public struct HealthDetailedResponse: Codable, Sendable {
    public let status: String?
    public let version: String?
    public let uptime: Double?
    public let memoryUsage: Int?
    public let cpuUsage: Double?
    public let components: [String: AnyCodable]?
}

// Document Extended
public struct DocumentChunksResponse: Codable, Sendable {
    public let documentId: String?
    public let chunks: [DocumentChunk]?
    public let total: Int?
}

public struct DocumentChunk: Codable, Sendable {
    public let id: String?
    public let content: String?
    public let chunkIndex: Int?
    public let tokens: Int?
}

public struct DocumentStatusResponse: Codable, Sendable {
    public let id: String?
    public let status: String?
    public let progress: Double?
    public let error: String?
}

public struct DocumentSearchResponse: Codable, Sendable {
    public let documents: [Document]?
    public let total: Int?
}

// Entity Extended
public struct MergeEntitiesResponse: Codable, Sendable {
    public let status: String?
    public let mergedEntity: Entity?
    public let removedEntities: [String]?
}

public struct EntityTypesResponse: Codable, Sendable {
    public let types: [String]?
    public let counts: [String: Int]?
}

// Relationship Extended
public struct CreateRelationshipRequest: Codable, Sendable {
    public let sourceEntity: String
    public let targetEntity: String
    public let relationshipType: String
    public let weight: Double?
    public let description: String?

    public init(
        sourceEntity: String, targetEntity: String, relationshipType: String, weight: Double? = nil,
        description: String? = nil
    ) {
        self.sourceEntity = sourceEntity
        self.targetEntity = targetEntity
        self.relationshipType = relationshipType
        self.weight = weight
        self.description = description
    }
}

public struct RelationshipTypesResponse: Codable, Sendable {
    public let types: [String]?
    public let counts: [String: Int]?
}

// Graph Extended
public struct GraphStatsResponse: Codable, Sendable {
    public let nodeCount: Int?
    public let edgeCount: Int?
    public let avgDegree: Double?
    public let density: Double?
    public let components: Int?
}

public struct SubgraphResponse: Codable, Sendable {
    public let nodes: [GraphNode]?
    public let edges: [GraphEdge]?
    public let depth: Int?
}

// Task Extended
public struct TaskStatus: Codable, Sendable {
    public let id: String?
    public let status: String?
    public let progress: Double?
    public let error: String?
    public let result: AnyCodable?
}

// API Key Extended
public struct CreateApiKeyResponse: Codable, Sendable {
    public let id: String?
    public let key: String?
    public let name: String?
    public let prefix: String?
    public let createdAt: String?
}

// Pipeline Extended
public struct ProcessingListResponse: Codable, Sendable {
    public let items: [ProcessingItem]?
    public let total: Int?
}

public struct ProcessingItem: Codable, Sendable {
    public let id: String?
    public let documentId: String?
    public let status: String?
    public let progress: Double?
    public let startedAt: String?
}

public struct PipelineConfig: Codable, Sendable {
    public let maxWorkers: Int?
    public let batchSize: Int?
    public let retryLimit: Int?
    public let timeout: Int?
}

// Model Extended
public struct ModelListResponse: Codable, Sendable {
    public let models: [ModelInfo]?
    public let total: Int?
}

public struct ModelInfo: Codable, Sendable {
    public let id: String?
    public let name: String?
    public let provider: String?
    public let contextLength: Int?
    public let capabilities: [String]?
}

public struct ModelConfig: Codable, Sendable {
    public let provider: String?
    public let model: String?
    public let temperature: Double?
    public let maxTokens: Int?
}

public struct ModelTestResult: Codable, Sendable {
    public let success: Bool?
    public let latencyMs: Int?
    public let error: String?
}

// Cost Extended
public struct DailyCost: Codable, Sendable {
    public let date: String?
    public let cost: Double?
    public let queryCount: Int?
    public let documentCount: Int?
}

public struct ProviderCost: Codable, Sendable {
    public let provider: String?
    public let cost: Double?
    public let percentage: Double?
}

public struct ModelCost: Codable, Sendable {
    public let model: String?
    public let provider: String?
    public let cost: Double?
    public let tokenCount: Int?
}

// Conversation Extended
public struct MessageInfo: Codable, Sendable {
    public let id: String?
    public let conversationId: String?
    public let role: String?
    public let content: String?
    public let tokensUsed: Int?
    public let createdAt: String?
}

// Auth Extended
public struct AuthTokenResponse: Codable, Sendable {
    public let accessToken: String?
    public let refreshToken: String?
    public let expiresIn: Int?
    public let tokenType: String?
    public let user: AuthUserResponse?
}

public struct AuthUserResponse: Codable, Sendable {
    public let id: String?
    public let email: String?
    public let name: String?
    public let role: String?
    public let tenantId: String?
}

// Workspace Extended
public struct WorkspaceInfo: Codable, Sendable {
    public let id: String?
    public let name: String?
    public let tenantId: String?
    public let settings: [String: AnyCodable]?
    public let createdAt: String?
    public let updatedAt: String?
}

public struct WorkspaceListResponse: Codable, Sendable {
    public let items: [WorkspaceInfo]?
    public let total: Int?
}

public struct WorkspaceStatsResponse: Codable, Sendable {
    public let documentCount: Int?
    public let entityCount: Int?
    public let relationshipCount: Int?
    public let queryCount: Int?
    public let storageUsed: Int?
}

// Shared Extended
public struct SharedLinkResponse: Codable, Sendable {
    public let id: String?
    public let url: String?
    public let resourceType: String?
    public let resourceId: String?
    public let expiresAt: String?
}

public struct SharedAccessResponse: Codable, Sendable {
    public let resourceType: String?
    public let resourceId: String?
    public let content: AnyCodable?
}
