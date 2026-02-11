package io.edgequake.sdk.resources;

import com.fasterxml.jackson.core.type.TypeReference;
import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

import java.util.List;

/**
 * Workspace operations.
 * WHY: List/Create are tenant-scoped: /api/v1/tenants/{tenant_id}/workspaces.
 * Get/Stats are direct: /api/v1/workspaces/{workspace_id}.
 */
public class WorkspaceService {

    private final HttpHelper http;

    public WorkspaceService(HttpHelper http) { this.http = http; }

    public List<WorkspaceInfo> listForTenant(String tenantId) {
        return http.get("/api/v1/tenants/" + tenantId + "/workspaces", null,
                new TypeReference<List<WorkspaceInfo>>() {});
    }

    public WorkspaceInfo createForTenant(String tenantId, CreateWorkspaceRequest request) {
        return http.post("/api/v1/tenants/" + tenantId + "/workspaces", request, WorkspaceInfo.class);
    }

    public WorkspaceInfo get(String id) {
        return http.get("/api/v1/workspaces/" + id, null, WorkspaceInfo.class);
    }

    public WorkspaceStats stats(String id) {
        return http.get("/api/v1/workspaces/" + id + "/stats", null, WorkspaceStats.class);
    }

    public RebuildResponse rebuildEmbeddings(String id) {
        return http.post("/api/v1/workspaces/" + id + "/rebuild-embeddings", null, RebuildResponse.class);
    }
}
