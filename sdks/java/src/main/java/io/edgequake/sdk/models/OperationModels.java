package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * Operation model classes: Pipeline, Tasks, Models, Costs, PDF.
 *
 * WHY: All field names verified against the real EdgeQuake API responses.
 */
public class OperationModels {

    // ── Pipeline ─────────────────────────────────────────────────────

    /**
     * WHY: Real API uses is_busy/pending_tasks/processing_tasks (not status/active/queued).
     * Verified via: curl http://localhost:8080/api/v1/pipeline/status
     */
    public static class PipelineStatus {
        @JsonProperty("is_busy") public boolean isBusy;
        @JsonProperty("total_documents") public int totalDocuments;
        @JsonProperty("processed_documents") public int processedDocuments;
        @JsonProperty("pending_tasks") public int pendingTasks;
        @JsonProperty("processing_tasks") public int processingTasks;
        @JsonProperty("completed_tasks") public int completedTasks;
        @JsonProperty("failed_tasks") public int failedTasks;
        @JsonProperty("cancellation_requested") public boolean cancellationRequested;
    }

    /**
     * WHY: Route is /api/v1/pipeline/queue-metrics (not /pipeline/metrics).
     */
    public static class QueueMetrics {
        @JsonProperty("queue_depth") public int queueDepth;
        @JsonProperty("processing") public int processing;
        @JsonProperty("completed_last_hour") public int completedLastHour;
        @JsonProperty("failed_last_hour") public int failedLastHour;
        @JsonProperty("avg_processing_time_ms") public Double avgProcessingTimeMs;
    }

    // ── Tasks ────────────────────────────────────────────────────────

    /** WHY: Task progress is an object (not a float). */
    public static class TaskProgress {
        @JsonProperty("current_step") public String currentStep;
        @JsonProperty("percent_complete") public int percentComplete;
        @JsonProperty("total_steps") public int totalSteps;
    }

    public static class TaskResult {
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunk_count") public int chunkCount;
        @JsonProperty("entity_count") public int entityCount;
        @JsonProperty("relationship_count") public int relationshipCount;
    }

    public static class TaskInfo {
        @JsonProperty("track_id") public String trackId;
        @JsonProperty("tenant_id") public String tenantId;
        @JsonProperty("workspace_id") public String workspaceId;
        @JsonProperty("task_type") public String taskType;
        @JsonProperty("status") public String status;
        @JsonProperty("created_at") public String createdAt;
        @JsonProperty("updated_at") public String updatedAt;
        @JsonProperty("started_at") public String startedAt;
        @JsonProperty("completed_at") public String completedAt;
        @JsonProperty("error_message") public String errorMessage;
        @JsonProperty("retry_count") public int retryCount;
        @JsonProperty("max_retries") public int maxRetries;
        @JsonProperty("progress") public TaskProgress progress;
        @JsonProperty("result") public TaskResult result;
    }

    public static class TaskListResponse {
        @JsonProperty("tasks") public List<TaskInfo> tasks;
        @JsonProperty("total") public int total;
    }

    // ── Models / Providers ───────────────────────────────────────────

    public static class ModelCapabilities {
        @JsonProperty("context_length") public int contextLength;
        @JsonProperty("max_output_tokens") public int maxOutputTokens;
        @JsonProperty("supports_vision") public boolean supportsVision;
        @JsonProperty("supports_function_calling") public boolean supportsFunctionCalling;
        @JsonProperty("supports_json_mode") public boolean supportsJsonMode;
        @JsonProperty("supports_streaming") public boolean supportsStreaming;
        @JsonProperty("supports_system_message") public boolean supportsSystemMessage;
        @JsonProperty("embedding_dimension") public int embeddingDimension;
    }

    public static class ModelCost {
        @JsonProperty("input_per_1k") public double inputPer1k;
        @JsonProperty("output_per_1k") public double outputPer1k;
        @JsonProperty("embedding_per_1k") public double embeddingPer1k;
    }

    public static class ModelInfo {
        @JsonProperty("name") public String name;
        @JsonProperty("display_name") public String displayName;
        @JsonProperty("model_type") public String modelType;
        @JsonProperty("description") public String description;
        @JsonProperty("deprecated") public boolean deprecated;
        @JsonProperty("capabilities") public ModelCapabilities capabilities;
        @JsonProperty("cost") public ModelCost cost;
        @JsonProperty("provider") public String provider;
        @JsonProperty("is_available") public boolean isAvailable;
    }

    public static class ProviderInfo {
        @JsonProperty("name") public String name;
        @JsonProperty("display_name") public String displayName;
        @JsonProperty("provider_type") public String providerType;
        @JsonProperty("enabled") public boolean enabled;
        @JsonProperty("priority") public int priority;
        @JsonProperty("description") public String description;
        @JsonProperty("models") public List<ModelInfo> models;
    }

    /** WHY: GET /api/v1/models returns {providers: [...]}. */
    public static class ProviderCatalog {
        @JsonProperty("providers") public List<ProviderInfo> providers;
    }

    /**
     * WHY: GET /api/v1/models/health returns a bare array of these.
     * Uses enabled/display_name/provider_type (not status/latency_ms).
     */
    public static class ProviderHealthInfo {
        @JsonProperty("name") public String name;
        @JsonProperty("display_name") public String displayName;
        @JsonProperty("provider_type") public String providerType;
        @JsonProperty("enabled") public boolean enabled;
        @JsonProperty("priority") public int priority;
        @JsonProperty("description") public String description;
        @JsonProperty("models") public List<ModelInfo> models;
    }

    /** WHY: Provider status is at /api/v1/settings/provider/status. */
    public static class ProviderStatus {
        @JsonProperty("current_provider") public String currentProvider;
        @JsonProperty("current_model") public String currentModel;
        @JsonProperty("status") public String status;
    }

    // ── Costs ────────────────────────────────────────────────────────

    public static class CostSummary {
        @JsonProperty("total_cost_usd") public double totalCostUsd;
        @JsonProperty("total_tokens") public long totalTokens;
        @JsonProperty("total_input_tokens") public long totalInputTokens;
        @JsonProperty("total_output_tokens") public long totalOutputTokens;
        @JsonProperty("document_count") public int documentCount;
        @JsonProperty("query_count") public int queryCount;
    }

    public static class CostEntry {
        @JsonProperty("date") public String date;
        @JsonProperty("cost_usd") public double costUsd;
        @JsonProperty("tokens") public long tokens;
        @JsonProperty("requests") public int requests;
    }

    public static class BudgetInfo {
        @JsonProperty("monthly_budget_usd") public Double monthlyBudgetUsd;
        @JsonProperty("current_spend_usd") public double currentSpendUsd;
        @JsonProperty("remaining_usd") public Double remainingUsd;
    }

    // ── Chunks / Provenance / Lineage ────────────────────────────────

    public static class ChunkDetail {
        @JsonProperty("id") public String id;
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("content") public String content;
        @JsonProperty("chunk_index") public Integer chunkIndex;
        @JsonProperty("token_count") public Integer tokenCount;
    }

    public static class ProvenanceRecord {
        @JsonProperty("entity_id") public String entityId;
        @JsonProperty("entity_name") public String entityName;
        @JsonProperty("document_id") public String documentId;
        @JsonProperty("chunk_id") public String chunkId;
        @JsonProperty("extraction_method") public String extractionMethod;
        @JsonProperty("confidence") public Double confidence;
    }

    public static class LineageNode {
        @JsonProperty("id") public String id;
        @JsonProperty("name") public String name;
        @JsonProperty("node_type") public String nodeType;
    }

    public static class LineageEdge {
        @JsonProperty("source") public String source;
        @JsonProperty("target") public String target;
        @JsonProperty("relationship") public String relationship;
    }

    public static class LineageGraph {
        @JsonProperty("nodes") public List<LineageNode> nodes;
        @JsonProperty("edges") public List<LineageEdge> edges;
        @JsonProperty("root_id") public String rootId;
    }

    // ── PDF ──────────────────────────────────────────────────────────

    /** WHY: PDF endpoints are under /api/v1/documents/pdf/. */
    public static class PdfProgressResponse {
        @JsonProperty("track_id") public String trackId;
        @JsonProperty("status") public String status;
        @JsonProperty("progress") public Double progress;
    }

    public static class PdfContentResponse {
        @JsonProperty("id") public String id;
        @JsonProperty("markdown") public String markdown;
    }
}
