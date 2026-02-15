/**
 * E2E: Query tool test.
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("query tool (e2e)", () => {
  let client: Client;
  let cleanup: () => Promise<void>;
  let serverUp: boolean;

  beforeAll(async () => {
    serverUp = await isServerRunning();
    if (!serverUp) return;
    const ctx = await createTestClient();
    client = ctx.client;
    cleanup = ctx.cleanup;
  });

  afterAll(async () => {
    if (cleanup) await cleanup();
  });

  it("should execute a hybrid query", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    const result = (await callTool(client, "query", {
      query: "What is EdgeQuake?",
      mode: "hybrid",
    })) as {
      answer: string;
      mode: string;
      sources: unknown[];
      stats: Record<string, unknown>;
    };

    expect(result).toHaveProperty("answer");
    expect(result).toHaveProperty("mode");
    expect(result).toHaveProperty("sources");
    expect(result).toHaveProperty("stats");
    expect(result.stats).toHaveProperty("total_time_ms");
  });

  it("should execute a naive query", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    const result = (await callTool(client, "query", {
      query: "What technologies are used?",
      mode: "naive",
    })) as { answer: string; mode: string };

    expect(result).toHaveProperty("answer");
    expect(result.mode).toBe("naive");
  });
});
