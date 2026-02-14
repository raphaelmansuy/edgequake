# OODA-24 Observe: C# SDK Lineage Models & Service

## Findings

- C# SDK had 16 services, 252-line Models.cs, ~170-line Services.cs
- NO lineage models or service existed in source code
- LineageTest.cs had 802 lines but tested OTHER services (Health, Entity, Graph, etc.)
- C# uses System.Text.Json with `JsonElement` for flexible types
- Primary constructor pattern: `public class FooService(HttpHelper http)`
- `HttpHelper.GetAsync<T>()` has `where T : class` constraint — `JsonElement` (struct) needs `GetRawAsync`
- .NET 10.0 target framework

## SDK State Before

| Metric                  | Value                                  |
| ----------------------- | -------------------------------------- |
| Services                | 16                                     |
| Lineage models          | 0                                      |
| Lineage service methods | 0                                      |
| Unit tests              | 79 (UnitTest) + 39 (LineageTest) = 118 |
| E2E tests               | 21 (skipped, need live backend)        |
