/**
 * EdgeQuake SDK client factory — singleton per MCP session.
 *
 * On first use, if no tenant/workspace is configured via environment variables,
 * the client auto-discovers the default tenant and workspace from the server.
 */
import { EdgeQuake } from "edgequake-sdk";
import { resolveConfig, type McpConfig } from "./config.js";

let _client: EdgeQuake | null = null;
let _config: McpConfig | null = null;
let _initialized = false;

/**
 * Get the EdgeQuake client. On first call, auto-discovers tenant/workspace
 * if not already configured. Must be awaited.
 */
export async function getClient(): Promise<EdgeQuake> {
  if (!_initialized) {
    _config = resolveConfig();

    // Create a bootstrap client to discover tenant/workspace if needed
    const bootstrap = new EdgeQuake({
      baseUrl: _config.baseUrl,
      apiKey: _config.apiKey,
    });

    // Auto-discover tenant
    if (!_config.defaultTenant) {
      try {
        const tenants = await bootstrap.tenants.list();
        if (tenants.length > 0) {
          _config.defaultTenant = tenants[0].id;
        }
      } catch {
        // Ignore — will fail later with a clear error when tools are called
      }
    }

    // Auto-discover workspace
    if (!_config.defaultWorkspace && _config.defaultTenant) {
      try {
        const workspaces = await bootstrap.tenants.listWorkspaces(
          _config.defaultTenant,
        );
        if (workspaces.length > 0) {
          _config.defaultWorkspace = workspaces[0].id;
        }
      } catch {
        // Ignore
      }
    }

    _client = new EdgeQuake({
      baseUrl: _config.baseUrl,
      apiKey: _config.apiKey,
      tenantId: _config.defaultTenant,
      workspaceId: _config.defaultWorkspace,
    });

    _initialized = true;
  }
  return _client!;
}

export function getConfig(): McpConfig {
  if (!_config) {
    _config = resolveConfig();
  }
  return _config;
}
