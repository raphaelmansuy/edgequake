/**
 * E2E: Health tool test.
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("health tool (e2e)", () => {
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

  it("should return healthy status", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }
    const result = (await callTool(client, "health")) as Record<
      string,
      unknown
    >;
    expect(result).toHaveProperty("status", "healthy");
    expect(result).toHaveProperty("components");
  });
});
