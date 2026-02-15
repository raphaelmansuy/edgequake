/**
 * E2E: Graph exploration tools test.
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("graph tools (e2e)", () => {
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

  it("should search entities", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    const result = await callTool(client, "graph_search_entities", {
      limit: 10,
    });
    expect(Array.isArray(result)).toBe(true);
  });

  it("should search entities with label filter", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    const result = await callTool(client, "graph_search_entities", {
      label: "TECHNOLOGY",
      limit: 5,
    });
    expect(Array.isArray(result)).toBe(true);
  });

  it("should search relationships", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    const result = await callTool(client, "graph_search_relationships", {
      limit: 10,
    });
    expect(Array.isArray(result)).toBe(true);
  });
});
