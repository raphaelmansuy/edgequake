/**
 * Pipeline resource — pipeline status and control.
 *
 * @module resources/pipeline
 * @see edgequake/crates/edgequake-api/src/handlers/pipeline.rs
 */

import type {
  CostEstimate,
  CostEstimateRequest,
  ModelPricing,
  PipelineStatus,
  QueueMetrics,
} from "../types/tasks.js";
import { Resource } from "./base.js";

export class PipelineResource extends Resource {
  /** Get pipeline processing status. */
  async status(): Promise<PipelineStatus> {
    return this._get("/api/v1/pipeline/status");
  }

  /** Cancel all running pipeline jobs. */
  async cancel(): Promise<void> {
    await this._post("/api/v1/pipeline/cancel");
  }

  /** Get queue metrics (pending, processing, failed counts). */
  async queueMetrics(): Promise<QueueMetrics> {
    return this._get("/api/v1/pipeline/queue-metrics");
  }

  /** Get model pricing information. */
  async pricing(): Promise<ModelPricing[]> {
    return this._get("/api/v1/pipeline/costs/pricing");
  }

  /** Estimate cost for a document processing request. */
  async estimateCost(request: CostEstimateRequest): Promise<CostEstimate> {
    return this._post("/api/v1/pipeline/costs/estimate", request);
  }
}
