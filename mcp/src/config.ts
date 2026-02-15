/**
 * Configuration resolved from environment variables.
 */
export interface McpConfig {
  baseUrl: string;
  apiKey?: string;
  defaultTenant?: string;
  defaultWorkspace?: string;
}

export function resolveConfig(): McpConfig {
  const config: McpConfig = {
    baseUrl: process.env.EDGEQUAKE_BASE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY,
    defaultTenant: process.env.EDGEQUAKE_DEFAULT_TENANT,
    defaultWorkspace: process.env.EDGEQUAKE_DEFAULT_WORKSPACE,
  };

  // Security warning: API key should be set for production URLs
  if (
    !config.apiKey &&
    (config.baseUrl.startsWith("https://") ||
      config.baseUrl.includes("production"))
  ) {
    console.warn(
      "[EdgeQuake MCP] ⚠️  WARNING: No API key configured for production URL.\n" +
        "  Set EDGEQUAKE_API_KEY environment variable to secure your deployment.\n" +
        "  See: https://github.com/raphaelmansuy/edgequake/tree/edgequake-main/mcp#security",
    );
  }

  return config;
}
