/**
 * Health tool — check EdgeQuake server health.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { getClient } from "../client.js";
import { formatError } from "../errors.js";

export function registerHealthTools(server: McpServer): void {
  server.tool(
    "health",
    "Check EdgeQuake server health and component status",
    {},
    async () => {
      try {
        const client = await getClient();
        const health = await client.health();
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(health, null, 2),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );
}
