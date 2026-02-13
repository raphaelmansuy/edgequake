# OODA-24 Orient: C# Lineage Gap Analysis

## Gap
C# SDK had zero lineage source code despite having 802 lines of tests for other services.
TypeScript, Java, and Kotlin SDKs already had 19 lineage models + 7 service methods.

## Approach
- Create LineageModels.cs with 19 model classes (separate file per SRP)
- Create LineageService.cs with 7 async methods
- Wire into EdgeQuakeClient.cs (16→17 services)
- Use `GetRawAsync` + `JsonDocument.Parse` for `ExportLineageAsync` (JsonElement is struct)
