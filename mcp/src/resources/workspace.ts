/**
 * MCP resources — expose EdgeQuake data for context injection.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { getClient } from "../client.js";

export function registerResources(server: McpServer): void {
  // Workspace stats resource
  server.resource(
    "workspace-stats",
    "edgequake://workspace/{workspace_id}/stats",
    async (uri) => {
      const match = uri.href.match(/edgequake:\/\/workspace\/([^/]+)\/stats/);
      if (!match) {
        return {
          contents: [
            { uri: uri.href, mimeType: "text/plain", text: "Invalid URI" },
          ],
        };
      }
      try {
        const workspaceId = match[1];
        const client = await getClient();
        const stats = await client.workspaces.stats(workspaceId);
        const detail = await client.workspaces.get(workspaceId);

        const text = [
          `# Workspace: ${detail.name}`,
          ``,
          `- **ID**: ${detail.id}`,
          `- **Description**: ${detail.description ?? "—"}`,
          `- **Documents**: ${stats.document_count}`,
          `- **Entities**: ${stats.entity_count}`,
          `- **Relationships**: ${stats.relationship_count}`,
          `- **Chunks**: ${stats.chunk_count}`,
          `- **LLM**: ${detail.llm_provider}/${detail.llm_model}`,
          `- **Embedding**: ${detail.embedding_provider}/${detail.embedding_model}`,
        ].join("\n");

        return {
          contents: [{ uri: uri.href, mimeType: "text/markdown", text }],
        };
      } catch (error) {
        const message =
          error instanceof Error ? error.message : String(error);
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "text/plain",
              text: `Error: ${message}`,
            },
          ],
        };
      }
    },
  );

  // Workspace entities resource
  server.resource(
    "workspace-entities",
    "edgequake://workspace/{workspace_id}/entities",
    async (uri) => {
      const match = uri.href.match(
        /edgequake:\/\/workspace\/([^/]+)\/entities/,
      );
      if (!match) {
        return {
          contents: [
            { uri: uri.href, mimeType: "text/plain", text: "Invalid URI" },
          ],
        };
      }

      try {
        const client = await getClient();
        const entities = await client.graph.entities.list({ per_page: 100 });

        const lines = entities.map(
          (e) => `- **${e.name}** (${e.label}): ${e.description ?? "—"}`,
        );

        const text = [
          `# Knowledge Graph Entities`,
          ``,
          `Total: ${entities.length} entities`,
          ``,
          ...lines,
        ].join("\n");

        return {
          contents: [{ uri: uri.href, mimeType: "text/markdown", text }],
        };
      } catch (error) {
        const message =
          error instanceof Error ? error.message : String(error);
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "text/plain",
              text: `Error: ${message}`,
            },
          ],
        };
      }
    },
  );
}
