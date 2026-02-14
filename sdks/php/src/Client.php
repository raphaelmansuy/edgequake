<?php

declare(strict_types=1);

namespace EdgeQuake;

/**
 * Main client for the EdgeQuake API.
 *
 *   $client = new Client(new Config());
 *   $health = $client->health->check();
 *
 * OODA-33: Added auth, workspaces, shared services.
 */
class Client
{
    public readonly HealthService $health;
    public readonly AuthService $auth;
    public readonly DocumentService $documents;
    public readonly EntityService $entities;
    public readonly RelationshipService $relationships;
    public readonly GraphService $graph;
    public readonly QueryService $query;
    public readonly ChatService $chat;
    public readonly TenantService $tenants;
    public readonly UserService $users;
    public readonly ApiKeyService $apiKeys;
    public readonly TaskService $tasks;
    public readonly PipelineService $pipeline;
    public readonly ModelService $models;
    public readonly CostService $costs;
    public readonly ConversationService $conversations;
    public readonly FolderService $folders;
    public readonly WorkspaceService $workspaces;
    public readonly SharedService $shared;
    public readonly LineageService $lineage;

    public function __construct(?Config $config = null)
    {
        $config ??= new Config();
        $http = new HttpHelper($config);

        $this->health        = new HealthService($http);
        $this->auth          = new AuthService($http);
        $this->documents     = new DocumentService($http);
        $this->entities      = new EntityService($http);
        $this->relationships = new RelationshipService($http);
        $this->graph         = new GraphService($http);
        $this->query         = new QueryService($http);
        $this->chat          = new ChatService($http);
        $this->tenants       = new TenantService($http);
        $this->users         = new UserService($http);
        $this->apiKeys       = new ApiKeyService($http);
        $this->tasks         = new TaskService($http);
        $this->pipeline      = new PipelineService($http);
        $this->models        = new ModelService($http);
        $this->costs         = new CostService($http);
        $this->conversations = new ConversationService($http);
        $this->folders       = new FolderService($http);
        $this->workspaces    = new WorkspaceService($http);
        $this->shared        = new SharedService($http);
        $this->lineage       = new LineageService($http);
    }
}
