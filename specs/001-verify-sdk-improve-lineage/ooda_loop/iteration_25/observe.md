# OODA-25 Observe: C# SDK Lineage Test Coverage

## Findings

- After OODA-24: C# SDK has LineageModels.cs (19 classes) + LineageService.cs (7 methods)
- Existing LineageTest.cs had 39 tests covering Health, Entity, Graph, etc. model deserialization
- NO tests existed for the new LineageService endpoints
- MockHttpMessageHandler uses `RequestRecord(Method, Url, Body)` — has `Url`, not `Path`
