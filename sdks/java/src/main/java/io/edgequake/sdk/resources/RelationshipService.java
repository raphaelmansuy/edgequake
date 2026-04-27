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
        if (perPage > 0) params.put("page_size", String.valueOf(perPage));
        return http.get("/api/v1/graph/relationships", params, RelationshipListResponse.class);
    }

    public Relationship create(CreateRelationshipRequest request) {
        return http.post("/api/v1/graph/relationships", request, Relationship.class);
    }

    // ── OODA-38: Added missing relationship methods ──────────────────

    /** Get relationship by ID. */
    public RelationshipDetailResponse get(String id) {
        return http.get("/api/v1/graph/relationships/" + id, null, RelationshipDetailResponse.class);
    }

    /** Delete relationship by ID. */
    public void delete(String id) {
        http.delete("/api/v1/graph/relationships/" + id);
    }
}
