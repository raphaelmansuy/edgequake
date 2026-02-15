/**
 * MCP prompts — reusable prompt templates for agents.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

export function registerPrompts(server: McpServer): void {
  // RAG query prompt
  server.prompt(
    "rag_query",
    "Construct an effective RAG query for EdgeQuake knowledge retrieval",
    {
      topic: z.string().describe("The subject to query about"),
      mode: z
        .enum(["naive", "local", "global", "hybrid", "mix"])
        .optional()
        .describe("Query mode (default: hybrid)"),
    },
    async (params) => {
      const mode = params.mode ?? "hybrid";
      return {
        messages: [
          {
            role: "user" as const,
            content: {
              type: "text" as const,
              text: [
                `Use the EdgeQuake 'query' tool to answer the following question.`,
                ``,
                `**Question**: ${params.topic}`,
                `**Query mode**: ${mode}`,
                ``,
                `Guidelines:`,
                `- Use 'hybrid' mode for questions that combine specific entity knowledge with broad themes`,
                `- Use 'local' mode for questions about specific entities and their direct relationships`,
                `- Use 'global' mode for thematic or summary questions across the whole corpus`,
                `- Use 'naive' mode for simple keyword-based retrieval`,
                `- Review the source references to verify the answer's grounding`,
              ].join("\n"),
            },
          },
        ],
      };
    },
  );

  // Document summary prompt
  server.prompt(
    "document_summary",
    "Summarize a document after upload by querying its extracted knowledge",
    {
      document_id: z.string().describe("The document UUID to summarize"),
    },
    async (params) => {
      return {
        messages: [
          {
            role: "user" as const,
            content: {
              type: "text" as const,
              text: [
                `Please summarize the document with ID: ${params.document_id}`,
                ``,
                `Steps:`,
                `1. Use 'document_get' to retrieve the document content`,
                `2. Use 'document_status' to check if processing is complete`,
                `3. If completed, use 'query' to ask "What are the main topics and key findings in this document?"`,
                `4. Use 'graph_search_entities' to find entities extracted from this document`,
                `5. Provide a structured summary with: key topics, entities found, and relationships discovered`,
              ].join("\n"),
            },
          },
        ],
      };
    },
  );
}
