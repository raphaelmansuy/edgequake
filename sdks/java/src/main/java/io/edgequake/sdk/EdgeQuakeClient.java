package io.edgequake.sdk;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.resources.*;

/**
 * Main entry point for the EdgeQuake Java SDK.
 *
 * <pre>{@code
 * var config = EdgeQuakeConfig.builder()
 *     .baseUrl("http://localhost:8080")
 *     .apiKey("my-api-key")
 *     .build();
 * var client = new EdgeQuakeClient(config);
 *
 * // Health check
 * var health = client.health().check();
 * System.out.println("Status: " + health.status);
 *
 * // List entities
 * var entities = client.entities().list(1, 10, null);
 * System.out.println("Total: " + entities.total);
 * }</pre>
 */
public class EdgeQuakeClient {

    private final HttpHelper http;
    private final HealthService healthService;
    private final DocumentService documentService;
    private final EntityService entityService;
    private final RelationshipService relationshipService;
    private final GraphService graphService;
    private final QueryService queryService;
    private final ChatService chatService;
    private final AuthService authService;
    private final UserService userService;
    private final ApiKeyService apiKeyService;
    private final TenantService tenantService;
    private final ConversationService conversationService;
    private final FolderService folderService;
    private final TaskService taskService;
    private final PipelineService pipelineService;
    private final ModelService modelService;
    private final WorkspaceService workspaceService;
    private final PdfService pdfService;
    private final CostService costService;
    private final LineageService lineageService;

    public EdgeQuakeClient(EdgeQuakeConfig config) {
        this.http = new HttpHelper(config);
        this.healthService = new HealthService(http);
        this.documentService = new DocumentService(http);
        this.entityService = new EntityService(http);
        this.relationshipService = new RelationshipService(http);
        this.graphService = new GraphService(http);
        this.queryService = new QueryService(http);
        this.chatService = new ChatService(http);
        this.authService = new AuthService(http);
        this.userService = new UserService(http);
        this.apiKeyService = new ApiKeyService(http);
        this.tenantService = new TenantService(http);
        this.conversationService = new ConversationService(http);
        this.folderService = new FolderService(http);
        this.taskService = new TaskService(http);
        this.pipelineService = new PipelineService(http);
        this.modelService = new ModelService(http);
        this.workspaceService = new WorkspaceService(http);
        this.pdfService = new PdfService(http);
        this.costService = new CostService(http);
        this.lineageService = new LineageService(http);
    }

    // ── Service accessors ────────────────────────────────────────────

    public HealthService health() { return healthService; }
    public DocumentService documents() { return documentService; }
    public EntityService entities() { return entityService; }
    public RelationshipService relationships() { return relationshipService; }
    public GraphService graph() { return graphService; }
    public QueryService query() { return queryService; }
    public ChatService chat() { return chatService; }
    public AuthService auth() { return authService; }
    public UserService users() { return userService; }
    public ApiKeyService apiKeys() { return apiKeyService; }
    public TenantService tenants() { return tenantService; }
    public ConversationService conversations() { return conversationService; }
    public FolderService folders() { return folderService; }
    public TaskService tasks() { return taskService; }
    public PipelineService pipeline() { return pipelineService; }
    public ModelService models() { return modelService; }
    public WorkspaceService workspaces() { return workspaceService; }
    public PdfService pdf() { return pdfService; }
    public CostService costs() { return costService; }
    public LineageService lineage() { return lineageService; }
}
