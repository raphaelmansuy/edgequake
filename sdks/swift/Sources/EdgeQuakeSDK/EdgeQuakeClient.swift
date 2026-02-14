import Foundation

/// Main client for the EdgeQuake API.
///
/// Usage:
/// ```swift
/// let client = EdgeQuakeClient(config: EdgeQuakeConfig(baseUrl: "http://localhost:8080"))
/// let health = try await client.health.check()
/// print(health.status ?? "unknown")
/// ```
public final class EdgeQuakeClient: @unchecked Sendable {
    private let http: HttpHelper

    public let health: HealthService
    public let documents: DocumentService
    public let entities: EntityService
    public let relationships: RelationshipService
    public let graph: GraphService
    public let query: QueryService
    public let chat: ChatService
    public let tenants: TenantService
    public let users: UserService
    public let apiKeys: ApiKeyService
    public let tasks: TaskService
    public let pipeline: PipelineService
    public let models: ModelService
    public let costs: CostService
    public let conversations: ConversationService
    public let folders: FolderService
    public let lineage: LineageService
    // OODA-35: New services
    public let auth: AuthService
    public let workspaces: WorkspaceService
    public let shared: SharedService

    public init(config: EdgeQuakeConfig = EdgeQuakeConfig()) {
        self.http = HttpHelper(config: config)
        self.health = HealthService(http)
        self.documents = DocumentService(http)
        self.entities = EntityService(http)
        self.relationships = RelationshipService(http)
        self.graph = GraphService(http)
        self.query = QueryService(http)
        self.chat = ChatService(http)
        self.tenants = TenantService(http)
        self.users = UserService(http)
        self.apiKeys = ApiKeyService(http)
        self.tasks = TaskService(http)
        self.pipeline = PipelineService(http)
        self.models = ModelService(http)
        self.costs = CostService(http)
        self.conversations = ConversationService(http)
        self.folders = FolderService(http)
        self.lineage = LineageService(http)
        // OODA-35: New services
        self.auth = AuthService(http)
        self.workspaces = WorkspaceService(http)
        self.shared = SharedService(http)
    }
}
