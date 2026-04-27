/**
 * Shared REST paths (DRY across resources).
 *
 * WHY: `PipelineResource` and `CostsResource` both expose model pricing and cost
 * estimation; the server mounts them only under `/api/v1/pipeline/costs/*`.
 */

export const PIPELINE_COSTS_PRICING_PATH = "/api/v1/pipeline/costs/pricing";

export const PIPELINE_COSTS_ESTIMATE_PATH = "/api/v1/pipeline/costs/estimate";
