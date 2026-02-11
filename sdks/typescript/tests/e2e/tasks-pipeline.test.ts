/**
 * E2E tests for Tasks, Pipeline, Settings, and Models resources.
 *
 * @module tests/e2e/tasks-pipeline
 * Tests against a live EdgeQuake backend:
 *   EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { beforeAll, describe, expect, it } from "vitest";
import type { EdgeQuake } from "../../src/index.js";
import { createE2EClient, E2E_ENABLED } from "./helpers.js";

describe.skipIf(!E2E_ENABLED)("Tasks E2E", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("lists tasks with pagination", { timeout: 15_000 }, async () => {
    const result = await client.tasks.list({ page: 1, page_size: 5 });
    expect(result).toHaveProperty("tasks");
    expect(result).toHaveProperty("pagination");
    expect(result).toHaveProperty("statistics");
    expect(Array.isArray(result.tasks)).toBe(true);
    expect(typeof result.pagination.total).toBe("number");
    expect(typeof result.statistics.pending).toBe("number");
  });

  it("lists tasks with status filter", { timeout: 15_000 }, async () => {
    const result = await client.tasks.list({
      status: "indexed",
      page_size: 3,
    });
    expect(result).toHaveProperty("tasks");
    // All returned tasks should have the requested status
    for (const task of result.tasks) {
      expect(task.status).toBe("indexed");
    }
  });

  it("gets a specific task by track_id", { timeout: 15_000 }, async () => {
    // First list to get a valid track_id
    const list = await client.tasks.list({ page_size: 1 });
    if (list.tasks.length === 0) {
      console.log("No tasks available — skipping");
      return;
    }

    const trackId = list.tasks[0].track_id;
    const task = await client.tasks.get(trackId);
    expect(task.track_id).toBe(trackId);
    expect(task).toHaveProperty("tenant_id");
    expect(task).toHaveProperty("workspace_id");
    expect(task).toHaveProperty("task_type");
    expect(task).toHaveProperty("status");
  });
});

describe.skipIf(!E2E_ENABLED)("Pipeline E2E", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("gets pipeline status", { timeout: 15_000 }, async () => {
    const status = await client.pipeline.status();
    expect(status).toHaveProperty("is_busy");
    expect(status).toHaveProperty("total_documents");
    expect(typeof status.is_busy).toBe("boolean");
  });

  it("gets queue metrics", { timeout: 15_000 }, async () => {
    try {
      const metrics = await client.pipeline.queueMetrics();
      expect(metrics).toBeDefined();
    } catch (err: unknown) {
      // Queue metrics may not be available
      console.log("Queue metrics not available:", String(err));
    }
  });
});

describe.skipIf(!E2E_ENABLED)("Settings E2E", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("gets provider status", { timeout: 15_000 }, async () => {
    const status = await client.settings.providerStatus();
    expect(status).toHaveProperty("provider");
    expect(status.provider).toHaveProperty("name");
    expect(status.provider).toHaveProperty("status");
  });

  it("lists available providers", { timeout: 15_000 }, async () => {
    const providers = await client.settings.listProviders();
    expect(providers).toBeDefined();
  });
});

describe.skipIf(!E2E_ENABLED)("Models E2E", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("lists all models", { timeout: 15_000 }, async () => {
    const models = await client.models.list();
    expect(models).toHaveProperty("providers");
    expect(Array.isArray(models.providers)).toBe(true);
    expect(models.providers.length).toBeGreaterThan(0);
    // First provider should have models
    expect(models.providers[0]).toHaveProperty("name");
    expect(models.providers[0]).toHaveProperty("models");
  });

  it("lists LLM models", { timeout: 15_000 }, async () => {
    const llmModels = await client.models.listLlm();
    expect(llmModels).toBeDefined();
  });

  it("lists embedding models", { timeout: 15_000 }, async () => {
    const embeddingModels = await client.models.listEmbedding();
    expect(embeddingModels).toBeDefined();
  });

  it("checks providers health", { timeout: 15_000 }, async () => {
    const health = await client.models.health();
    expect(health).toBeDefined();
  });
});
