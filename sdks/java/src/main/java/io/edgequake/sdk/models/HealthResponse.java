package io.edgequake.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;

/** HealthResponse from GET /health. */
public class HealthResponse {
    @JsonProperty("status") public String status;
    @JsonProperty("version") public String version;
    @JsonProperty("storage_mode") public String storageMode;
    @JsonProperty("workspace_id") public String workspaceId;
    @JsonProperty("llm_provider_name") public String llmProviderName;
}
