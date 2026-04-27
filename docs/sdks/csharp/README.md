# C# / .NET SDK

**Location:** `sdks/csharp`

## Example

```csharp
using EdgeQuakeSDK;

var http = new HttpHelper(new EdgeQuakeConfig {
    BaseUrl = "http://localhost:8080",
    ApiKey = Environment.GetEnvironmentVariable("EDGEQUAKE_API_KEY"),
    TenantId = Environment.GetEnvironmentVariable("EDGEQUAKE_TENANT_ID"),
    UserId = Environment.GetEnvironmentVariable("EDGEQUAKE_USER_ID"),
    WorkspaceId = Environment.GetEnvironmentVariable("EDGEQUAKE_WORKSPACE_ID"),
});

var health = await new HealthService(http).CheckAsync();
Console.WriteLine(health.Status);

var bulk = await new ConversationService(http).BulkDeleteAsync(new List<string> { "c1", "c2" });
Console.WriteLine(bulk.Affected);
```

`BulkDeleteAsync` posts a body with **`conversation_ids`**.

## Test

```bash
cd sdks/csharp && dotnet test
```
