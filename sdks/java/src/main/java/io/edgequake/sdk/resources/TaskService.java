package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.OperationModels.*;

import java.util.LinkedHashMap;
import java.util.Map;

/** Task operations at /api/v1/tasks. */
public class TaskService {

    private final HttpHelper http;

    public TaskService(HttpHelper http) { this.http = http; }

    public TaskListResponse list(String status, int page, int perPage) {
        Map<String, String> params = new LinkedHashMap<>();
        if (status != null && !status.isEmpty()) params.put("status", status);
        if (page > 0) params.put("page", String.valueOf(page));
        if (perPage > 0) params.put("per_page", String.valueOf(perPage));
        return http.get("/api/v1/tasks", params, TaskListResponse.class);
    }

    public TaskInfo get(String trackId) {
        return http.get("/api/v1/tasks/" + trackId, null, TaskInfo.class);
    }

    public void cancel(String trackId) {
        http.postNoContent("/api/v1/tasks/" + trackId + "/cancel", null);
    }
}
