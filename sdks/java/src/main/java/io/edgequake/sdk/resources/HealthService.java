package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.HealthResponse;

/** Health endpoint at root /health (not under /api/v1/). */
public class HealthService {

    private final HttpHelper http;

    public HealthService(HttpHelper http) { this.http = http; }

    public HealthResponse check() {
        return http.get("/health", null, HealthResponse.class);
    }
}
