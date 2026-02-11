package io.edgequake.sdk;

/**
 * Configuration for the EdgeQuake client. Uses builder pattern.
 *
 * <pre>{@code
 * var config = EdgeQuakeConfig.builder()
 *     .baseUrl("http://localhost:8080")
 *     .apiKey("my-api-key")
 *     .build();
 * var client = new EdgeQuakeClient(config);
 * }</pre>
 */
public class EdgeQuakeConfig {

    private final String baseUrl;
    private final String apiKey;
    private final String tenantId;
    private final String userId;
    private final String workspaceId;
    private final int timeoutSeconds;

    private EdgeQuakeConfig(Builder builder) {
        this.baseUrl = builder.baseUrl;
        this.apiKey = builder.apiKey;
        this.tenantId = builder.tenantId;
        this.userId = builder.userId;
        this.workspaceId = builder.workspaceId;
        this.timeoutSeconds = builder.timeoutSeconds;
    }

    public String baseUrl() { return baseUrl; }
    public String apiKey() { return apiKey; }
    public String tenantId() { return tenantId; }
    public String userId() { return userId; }
    public String workspaceId() { return workspaceId; }
    public int timeoutSeconds() { return timeoutSeconds; }

    public static Builder builder() { return new Builder(); }

    public static class Builder {
        private String baseUrl = "http://localhost:8080";
        private String apiKey;
        private String tenantId;
        private String userId;
        private String workspaceId;
        private int timeoutSeconds = 30;

        public Builder baseUrl(String baseUrl) { this.baseUrl = baseUrl; return this; }
        public Builder apiKey(String apiKey) { this.apiKey = apiKey; return this; }
        public Builder tenantId(String tenantId) { this.tenantId = tenantId; return this; }
        public Builder userId(String userId) { this.userId = userId; return this; }
        public Builder workspaceId(String workspaceId) { this.workspaceId = workspaceId; return this; }
        public Builder timeoutSeconds(int timeoutSeconds) { this.timeoutSeconds = timeoutSeconds; return this; }

        public EdgeQuakeConfig build() {
            return new EdgeQuakeConfig(this);
        }
    }
}
