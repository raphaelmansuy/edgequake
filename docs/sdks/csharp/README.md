---
title: C# SDK
description: Use the EdgeQuake .NET client (EdgeQuake.SDK 0.4.0, net10.0). Not on NuGet yet — reference the project from sdks/csharp.
---

# C# SDK

.NET client for EdgeQuake. Package id: **`EdgeQuake.SDK`** version **0.4.0**. Targets **`net10.0`**. **Not published** on NuGet; reference `sdks/csharp/src/EdgeQuakeSDK/EdgeQuakeSDK.csproj`.

```csharp
using EdgeQuakeSDK;

var client = new EdgeQuakeClient(new EdgeQuakeConfig {
    BaseUrl = "http://localhost:8080",
    ApiKey = "eq-...",
});

var docs = await client.Documents.ListAsync(page: 1, pageSize: 20);
var answer = await client.Query.ExecuteAsync("What is in my documents?", mode: "mix");
Console.WriteLine(answer.Answer);
```

Class: `EdgeQuakeClient` with `Documents`, `Query`, `Entities`, and other services. Connections are not wrapped. See [SDK overview](../README.md).
