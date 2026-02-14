package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.QueryModels.*;

/** Query operations at /api/v1/query. */
public class QueryService {

    private final HttpHelper http;

    public QueryService(HttpHelper http) { this.http = http; }

    public QueryResponse execute(QueryRequest request) {
        return http.post("/api/v1/query", request, QueryResponse.class);
    }

    // ── OODA-38: Added streaming query method ────────────────────────

    /** Stream query response (SSE). */
    public String stream(String query) {
        var request = new QueryRequest();
        request.query = query;
        return http.postRaw("/api/v1/query/stream", request);
    }
}
