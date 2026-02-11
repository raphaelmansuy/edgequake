import Foundation

/// Configuration for the EdgeQuake client.
public struct EdgeQuakeConfig {
    public let baseUrl: String
    public let apiKey: String?
    public let tenantId: String?
    public let userId: String?
    public let workspaceId: String?
    public let timeoutSeconds: TimeInterval

    public init(
        baseUrl: String = "http://localhost:8080",
        apiKey: String? = nil,
        tenantId: String? = nil,
        userId: String? = nil,
        workspaceId: String? = nil,
        timeoutSeconds: TimeInterval = 30
    ) {
        self.baseUrl = baseUrl
        self.apiKey = apiKey
        self.tenantId = tenantId
        self.userId = userId
        self.workspaceId = workspaceId
        self.timeoutSeconds = timeoutSeconds
    }
}
