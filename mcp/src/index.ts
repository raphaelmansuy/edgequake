/**
 * EdgeQuake MCP Server — entry point.
 *
 * Exposes EdgeQuake Graph-RAG as an MCP server over stdio transport.
 * Agents can manage workspaces, upload documents, query knowledge graphs,
 * and explore entities through standard MCP tools.
 */
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createServer } from "./server.js";

async function main(): Promise<void> {
  const server = createServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((error) => {
  console.error("EdgeQuake MCP server failed to start:", error);
  process.exit(1);
});
