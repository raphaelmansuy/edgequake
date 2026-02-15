/**
 * MCP server setup — creates the McpServer instance with all tools, resources, and prompts.
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerPrompts } from "./prompts/rag.js";
import { registerResources } from "./resources/workspace.js";
import { registerDocumentTools } from "./tools/document.js";
import { registerGraphTools } from "./tools/graph.js";
import { registerHealthTools } from "./tools/health.js";
import { registerQueryTools } from "./tools/query.js";
import { registerWorkspaceTools } from "./tools/workspace.js";

export function createServer(): McpServer {
  const server = new McpServer({
    name: "edgequake",
    version: "0.1.0",
  });

  registerHealthTools(server);
  registerWorkspaceTools(server);
  registerDocumentTools(server);
  registerQueryTools(server);
  registerGraphTools(server);
  registerResources(server);
  registerPrompts(server);

  return server;
}
