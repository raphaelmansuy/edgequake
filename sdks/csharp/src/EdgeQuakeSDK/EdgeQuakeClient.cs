namespace EdgeQuakeSDK;

/// <summary>Main client for the EdgeQuake API.</summary>
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
    }
}
