# frozen_string_literal: true

module EdgeQuake
  # Main client for the EdgeQuake API.
  # OODA-34: Added auth, workspaces, shared services for complete API coverage.
  #
  #   client = EdgeQuake::Client.new(config: EdgeQuake::Config.new)
  #   health = client.health.check
  #   puts health["status"]
  #
  class Client
    attr_reader :health, :documents, :entities, :relationships, :graph,
                :query, :chat, :tenants, :users, :api_keys, :tasks,
                :pipeline, :models, :costs, :conversations, :folders, :lineage,
                :auth, :workspaces, :shared

    def initialize(config: Config.new)
      http = HttpHelper.new(config)
      @health        = HealthService.new(http)
      @documents     = DocumentService.new(http)
      @entities      = EntityService.new(http)
      @relationships = RelationshipService.new(http)
      @graph         = GraphService.new(http)
      @query         = QueryService.new(http)
      @chat          = ChatService.new(http)
      @tenants       = TenantService.new(http)
      @users         = UserService.new(http)
      @api_keys      = ApiKeyService.new(http)
      @tasks         = TaskService.new(http)
      @pipeline      = PipelineService.new(http)
      @models        = ModelService.new(http)
      @costs         = CostService.new(http)
      @conversations = ConversationService.new(http)
      @folders       = FolderService.new(http)
      @lineage       = LineageService.new(http)
      @auth          = AuthService.new(http)
      @workspaces    = WorkspaceService.new(http)
      @shared        = SharedService.new(http)
    end
  end
end
