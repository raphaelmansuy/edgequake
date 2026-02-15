/**
 * Query tool — the primary tool for agents to retrieve knowledge.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getClient } from "../client.js";
import { formatError } from "../errors.js";

export function registerQueryTools(server: McpServer): void {
  server.tool(
    "query",
    "Execute a RAG query against the EdgeQuake knowledge graph. Returns an AI-generated answer with source references. Use 'hybrid' mode (default) for best results combining local entity graph traversal with global semantic search.",
    {
      query: z.string().describe("Natural language question"),
      mode: z
        .enum(["naive", "local", "global", "hybrid", "mix"])
        .optional()
        .describe(
          "Query mode: naive (vector-only), local (entity graph), global (community search), hybrid (local+global, default), mix (weighted blend)",
        ),
      max_results: z
        .number()
        .optional()
        .describe("Maximum number of source references to return"),
      include_references: z
        .boolean()
        .optional()
        .describe("Include source snippets in response (default: true)"),
      conversation_history: z
        .array(
          z.object({
            role: z.enum(["user", "assistant", "system"]),
            content: z.string(),
          }),
        )
        .optional()
        .describe("Prior conversation messages for multi-turn context"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const result = await client.query.execute({
          query: params.query,
          mode: params.mode,
          max_results: params.max_results,
          include_references: params.include_references ?? true,
          conversation_history: params.conversation_history,
        });

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  answer: result.answer,
                  mode: result.mode,
                  sources: result.sources.map((s) => ({
                    source_type: s.source_type,
                    snippet: s.snippet,
                    score: s.score,
                    document_id: s.document_id,
                  })),
                  stats: {
                    total_time_ms: result.stats.total_time_ms,
                    sources_retrieved: result.stats.sources_retrieved,
                  },
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );
}
