package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.HealthResponse;
import io.edgequake.sdk.models.ReadinessResponse;
import io.edgequake.sdk.models.LivenessResponse;

/** Health endpoint at root /health (not under /api/v1/). */
public class HealthService {

    private final HttpHelper http;

    public HealthService(HttpHelper http) { this.http = http; }

    public HealthResponse check() {
        return http.get("/health", null, HealthResponse.class);
    }

    /** Readiness check at /ready. */
    public ReadinessResponse ready() {
        return http.get("/ready", null, ReadinessResponse.class);
    }

    /** Liveness check at /live. */
    public LivenessResponse live() {
        return http.get("/live", null, LivenessResponse.class);
    }

    /** Prometheus metrics at /metrics. */
    public String metrics() {
        return http.getRaw("/metrics", null);
    }
}
