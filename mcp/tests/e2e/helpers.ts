/**
 * E2E test helpers — create an MCP client connected to the EdgeQuake MCP server.
 */
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "../../src/server.js";

/**
 * Creates a connected MCP client+server pair for E2E testing.
 * Uses in-memory transport — no stdio needed.
 * Requires EDGEQUAKE_BASE_URL to point to a running EdgeQuake server.
 */
export async function createTestClient(): Promise<{
  client: Client;
  cleanup: () => Promise<void>;
}> {
  const server = createServer();
  const client = new Client({ name: "test-client", version: "0.1.0" });

  const [clientTransport, serverTransport] =
    InMemoryTransport.createLinkedPair();

  await Promise.all([
    server.connect(serverTransport),
    client.connect(clientTransport),
  ]);

  return {
    client,
    cleanup: async () => {
      await client.close();
      await server.close();
    },
  };
}

/**
 * Call an MCP tool and return the parsed text content.
 */
export async function callTool(
  client: Client,
  name: string,
  args: Record<string, unknown> = {},
): Promise<unknown> {
  const result = await client.callTool({ name, arguments: args });
  const textContent = result.content as Array<{ type: string; text: string }>;
  const text = textContent?.[0]?.text;
  if (!text) return result;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

/**
 * Check if EdgeQuake server is reachable.
 */
export async function isServerRunning(): Promise<boolean> {
  const baseUrl = process.env.EDGEQUAKE_BASE_URL ?? "http://localhost:8080";
  try {
    const res = await fetch(`${baseUrl}/health`);
    return res.ok;
  } catch {
    return false;
  }
}
