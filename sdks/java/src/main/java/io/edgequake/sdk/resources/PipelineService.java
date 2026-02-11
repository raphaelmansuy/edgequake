package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.OperationModels.*;

/**
 * Pipeline operations at /api/v1/pipeline.
 *
 * WHY: Pipeline status uses is_busy/pending_tasks/processing_tasks fields.
 * Queue metrics are at /pipeline/queue-metrics (not /pipeline/metrics).
 */
public class PipelineService {

    private final HttpHelper http;

    public PipelineService(HttpHelper http) { this.http = http; }

    public PipelineStatus status() {
        return http.get("/api/v1/pipeline/status", null, PipelineStatus.class);
    }

    /** WHY: Route is /api/v1/pipeline/queue-metrics. */
    public QueueMetrics metrics() {
        return http.get("/api/v1/pipeline/queue-metrics", null, QueueMetrics.class);
    }
}
