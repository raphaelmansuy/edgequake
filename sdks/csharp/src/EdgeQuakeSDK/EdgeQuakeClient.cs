namespace EdgeQuakeSDK;

/// <summary>Main client for the EdgeQuake API.</summary>
/// <remarks>OODA-36: Added AuthService, WorkspaceService, SharedService.</remarks>
public class EdgeQuakeClient
{
    public HealthService Health { get; }
    public DocumentService Documents { get; }
    public EntityService Entities { get; }
    public RelationshipService Relationships { get; }
    public GraphService Graph { get; }
    public QueryService Query { get; }
    public ChatService Chat { get; }
    public TenantService Tenants { get; }
    public UserService Users { get; }
    public ApiKeyService ApiKeys { get; }
    public TaskService Tasks { get; }
    public PipelineService Pipeline { get; }
    public ModelService Models { get; }
    public CostService Costs { get; }
    public ConversationService Conversations { get; }
    public FolderService Folders { get; }
    /// <summary>WHY: Lineage service added in OODA-24 for entity/document/chunk provenance.</summary>
    public LineageService Lineage { get; }
    /// <summary>WHY: Auth service added in OODA-36 for login/logout/refresh/me.</summary>
    public AuthService Auth { get; }
    /// <summary>WHY: Workspace service added in OODA-36 for workspace CRUD/stats/switch.</summary>
    public WorkspaceService Workspaces { get; }
    /// <summary>WHY: Shared service added in OODA-36 for conversation sharing.</summary>
    public SharedService Shared { get; }

    public EdgeQuakeClient(EdgeQuakeConfig? config = null)
    {
        config ??= new EdgeQuakeConfig();
        var http = new HttpHelper(config);

        Health = new HealthService(http);
        Documents = new DocumentService(http);
        Entities = new EntityService(http);
        Relationships = new RelationshipService(http);
        Graph = new GraphService(http);
        Query = new QueryService(http);
        Chat = new ChatService(http);
        Tenants = new TenantService(http);
        Users = new UserService(http);
        ApiKeys = new ApiKeyService(http);
        Tasks = new TaskService(http);
        Pipeline = new PipelineService(http);
        Models = new ModelService(http);
        Costs = new CostService(http);
        Conversations = new ConversationService(http);
        Folders = new FolderService(http);
        Lineage = new LineageService(http);
        Auth = new AuthService(http);
        Workspaces = new WorkspaceService(http);
        Shared = new SharedService(http);
    }
}
