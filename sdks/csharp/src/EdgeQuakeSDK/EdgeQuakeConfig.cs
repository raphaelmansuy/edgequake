namespace EdgeQuakeSDK;

/// <summary>Configuration for the EdgeQuake client.</summary>
public class EdgeQuakeConfig
{
    public string BaseUrl { get; set; } = "http://localhost:8080";
    public string? ApiKey { get; set; }
    public string? TenantId { get; set; }
    public string? UserId { get; set; }
    public string? WorkspaceId { get; set; }
    public int TimeoutSeconds { get; set; } = 60;
}
