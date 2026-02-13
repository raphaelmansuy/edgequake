# OODA-24 Decide: C# Lineage Implementation Plan

1. Create `LineageModels.cs` — 19 model classes matching Rust lineage_types.rs
2. Create `LineageService.cs` — 7 async methods for lineage endpoints
3. Wire `LineageService` into `EdgeQuakeClient.cs`
4. Fix `ExportLineageAsync` to use `GetRawAsync` (JsonElement is struct, not class)
5. Update service count test from 16 to 17
6. Build + test to verify zero regressions
