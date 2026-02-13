# Analysis - Iteration 28

## Recommendation
Register all 15 lineage DTO types in OpenAPI `components(schemas())`. Add `ToSchema` derive to `ExportParams`. This ensures the Swagger UI and SDK code generators have complete type information for all lineage endpoints.
