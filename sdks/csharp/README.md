# EdgeQuake C# SDK

Official .NET client for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG API.

## Requirements

- .NET 10.0+
- No external NuGet dependencies

## Installation

```bash
dotnet add package EdgeQuakeSDK
```

## Quick Start

```csharp
using EdgeQuakeSDK;

var client = new EdgeQuakeClient(new EdgeQuakeConfig
{
    BaseUrl = "http://localhost:8080",
    ApiKey  = "sk-your-key",
});

// Health check
var health = await client.Health.CheckAsync();
Console.WriteLine(health.Status);  // "healthy"

// Upload a document
var doc = await client.Documents.UploadTextAsync("My Doc", "Hello, world!");
Console.WriteLine(doc.DocumentId);

// Query the knowledge graph
var result = await client.Query.ExecuteAsync("What do you know?", "hybrid");
Console.WriteLine(result.Answer);
```

## Configuration

| Property         | Type      | Default                 | Description           |
| ---------------- | --------- | ----------------------- | --------------------- |
| `BaseUrl`        | `string`  | `http://localhost:8080` | API base URL          |
| `ApiKey`         | `string?` | `null`                  | API key               |
| `TenantId`       | `string?` | `null`                  | Tenant ID header      |
| `UserId`         | `string?` | `null`                  | User ID header        |
| `WorkspaceId`    | `string?` | `null`                  | Workspace ID header   |
| `TimeoutSeconds` | `int`     | `60`                    | Request timeout (sec) |

```csharp
var config = new EdgeQuakeConfig
{
    BaseUrl        = "https://api.example.com",
    ApiKey         = "sk-my-key",
    TenantId       = "tenant-1",
    UserId         = "user-1",
    WorkspaceId    = "ws-1",
    TimeoutSeconds = 120,
};
var client = new EdgeQuakeClient(config);
```

## Services

All service methods are **async** and return `Task<T>`.

### Health

```csharp
var status = await client.Health.CheckAsync();
// HealthResponse { Status, Version, StorageMode, ... }
```

### Documents

```csharp
// List
var docs = await client.Documents.ListAsync(page: 1, pageSize: 20);

// Upload text
var doc = await client.Documents.UploadTextAsync("Title", "Content", "txt");

// Delete
await client.Documents.DeleteAsync("doc-id");
```

### Entities

```csharp
// List
var entities = await client.Entities.ListAsync(page: 1, pageSize: 20);

// Get
var entity = await client.Entities.GetAsync("ENTITY_NAME");

// Create
await client.Entities.CreateAsync("NODE", "concept", "A concept", "src-1");

// Delete
await client.Entities.DeleteAsync("NODE");
```

### Relationships

```csharp
var rels = await client.Relationships.ListAsync(page: 1, pageSize: 20);
```

### Graph

```csharp
var graph = await client.Graph.GetAsync();
var results = await client.Graph.SearchAsync("Alice");
```

### Query

```csharp
var result = await client.Query.ExecuteAsync("What is EdgeQuake?", "hybrid");
Console.WriteLine(result.Answer);
```

Modes: `hybrid`, `local`, `global`, `naive`.

### Chat

```csharp
var response = await client.Chat.CompletionsAsync("Hello!", "hybrid", stream: false);
Console.WriteLine(response.Content);
```

### Tenants / Users / API Keys / Tasks

```csharp
var tenants = await client.Tenants.ListAsync();
var users   = await client.Users.ListAsync();
var keys    = await client.ApiKeys.ListAsync();
var tasks   = await client.Tasks.ListAsync();
```

### Pipeline

```csharp
var status  = await client.Pipeline.StatusAsync();
var metrics = await client.Pipeline.QueueMetricsAsync();
```

### Models

```csharp
var catalog  = await client.Models.CatalogAsync();
var health   = await client.Models.HealthAsync();
var provider = await client.Models.ProviderStatusAsync();
```

### Costs

```csharp
var costs = await client.Costs.SummaryAsync();
Console.WriteLine(costs.TotalCost);
```

## Error Handling

All API errors throw `EdgeQuakeException` (subclass of `Exception`):

```csharp
try
{
    await client.Documents.DeleteAsync("nonexistent");
}
catch (EdgeQuakeException ex)
{
    Console.WriteLine(ex.Message);       // "HTTP 404: ..."
    Console.WriteLine(ex.StatusCode);    // 404
    Console.WriteLine(ex.ResponseBody);  // raw JSON
}
```

| Property       | Type      | Description       |
| -------------- | --------- | ----------------- |
| `StatusCode`   | `int?`    | HTTP status code  |
| `ResponseBody` | `string?` | Raw response body |
| `Message`      | `string`  | Error description |

## Testing

```bash
dotnet test
```

## Project Structure

```
src/EdgeQuakeSDK/
├── EdgeQuakeClient.cs      # Main client with service properties
├── EdgeQuakeConfig.cs      # Configuration
├── EdgeQuakeException.cs   # Error class
├── HttpHelper.cs           # HttpClient-based helper
├── Models.cs               # Response model classes
└── Services.cs             # 14 service classes
tests/EdgeQuakeSDK.Tests/
├── MockHttpMessageHandler.cs  # Mock for unit testing
├── UnitTest.cs                # 71 unit tests
└── E2ETest.cs                 # Integration tests
```

## License

MIT
