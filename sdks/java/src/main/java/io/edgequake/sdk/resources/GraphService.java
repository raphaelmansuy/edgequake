package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.GraphModels.*;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Graph operations at /api/v1/graph. */
public class GraphService {

    private final HttpHelper http;

    public GraphService(HttpHelper http) { this.http = http; }

    public GraphResponse get(int limit) {
        Map<String, String> params = new LinkedHashMap<>();
        if (limit > 0) params.put("limit", String.valueOf(limit));
        return http.get("/api/v1/graph", params, GraphResponse.class);
    }

    /**
     * Search graph nodes.
     * WHY: Uses /api/v1/graph/nodes/search with "q" query param (not "query").
     */
    public SearchNodesResponse search(String query, int limit) {
        Map<String, String> params = new LinkedHashMap<>();
        params.put("q", query);
        if (limit > 0) params.put("limit", String.valueOf(limit));
        return http.get("/api/v1/graph/nodes/search", params, SearchNodesResponse.class);
    }

    /** Search labels. */
    public LabelSearchResponse labelSearch(String query) {
        Map<String, String> params = new LinkedHashMap<>();
        params.put("q", query);
        return http.get("/api/v1/graph/labels/search", params, LabelSearchResponse.class);
    }

    /** Get popular labels. */
    public PopularLabelsResponse popularLabels() {
        return http.get("/api/v1/graph/labels/popular", null, PopularLabelsResponse.class);
    }

    /** Get degrees for multiple nodes in batch. */
    public BatchDegreesResponse batchDegrees(List<String> nodeIds) {
        Map<String, Object> body = new LinkedHashMap<>();
        body.put("node_ids", nodeIds);
        return http.post("/api/v1/graph/degrees/batch", body, BatchDegreesResponse.class);
    }
}
