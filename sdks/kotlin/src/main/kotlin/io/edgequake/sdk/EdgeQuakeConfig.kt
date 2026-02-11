package io.edgequake.sdk

/**
 * Configuration for the EdgeQuake client.
 * WHY: Data class with defaults = clean builder pattern for free.
 */
data class EdgeQuakeConfig(
    val baseUrl: String = "http://localhost:8080",
    val apiKey: String? = null,
    val tenantId: String? = null,
    val userId: String? = null,
    val workspaceId: String? = null,
    val timeoutSeconds: Long = 30
)
