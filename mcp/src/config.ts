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
  return {
    baseUrl: process.env.EDGEQUAKE_BASE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY,
    defaultTenant: process.env.EDGEQUAKE_DEFAULT_TENANT,
    defaultWorkspace: process.env.EDGEQUAKE_DEFAULT_WORKSPACE,
  };
}
