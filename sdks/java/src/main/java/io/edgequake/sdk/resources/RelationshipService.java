package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.GraphModels.*;

import java.util.LinkedHashMap;
import java.util.Map;

/** Relationship operations at /api/v1/graph/relationships. */
public class RelationshipService {

    private final HttpHelper http;

    public RelationshipService(HttpHelper http) { this.http = http; }

    public RelationshipListResponse list(int page, int perPage) {
        Map<String, String> params = new LinkedHashMap<>();
        if (page > 0) params.put("page", String.valueOf(page));
        if (perPage > 0) params.put("per_page", String.valueOf(perPage));
        return http.get("/api/v1/graph/relationships", params, RelationshipListResponse.class);
    }

    public Relationship create(CreateRelationshipRequest request) {
        return http.post("/api/v1/graph/relationships", request, Relationship.class);
    }
}
