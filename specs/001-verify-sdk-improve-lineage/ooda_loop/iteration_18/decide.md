# OODA-18: Decide — Create LineageModels.java

## Plan

1. Create `sdks/java/src/main/java/io/edgequake/sdk/models/LineageModels.java` with 19 model classes
2. All `@JsonProperty` annotations match backend field names exactly
3. Keep existing OperationModels lineage types for backward compatibility
4. Run `mvn test` to verify no regressions
