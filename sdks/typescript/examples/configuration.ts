/**
 * Example: Configuration Patterns
 *
 * Demonstrates different ways to configure the EdgeQuake SDK client,
 * including environment variables, explicit config, and multi-environment setup.
 */

import { EdgeQuake } from "@edgequake/sdk";

// --- Pattern 1: Minimal configuration ---
// WHY: baseUrl defaults to EDGEQUAKE_BASE_URL env var or http://localhost:8080
const simpleClient = new EdgeQuake();

// --- Pattern 2: Explicit configuration ---
const explicitClient = new EdgeQuake({
  baseUrl: "https://api.edgequake.example.com",
  apiKey: "your-api-key-here",
  timeout: 30_000, // 30 seconds
});

// --- Pattern 3: Environment-based configuration ---
// Set these environment variables:
//   EDGEQUAKE_BASE_URL=https://api.edgequake.example.com
//   EDGEQUAKE_API_KEY=sk-...
//   EDGEQUAKE_WORKSPACE_ID=my-workspace
const envClient = new EdgeQuake({
  // SDK reads from environment variables automatically
});

// --- Pattern 4: Multi-tenant configuration ---
const tenantClient = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "admin-api-key",
  workspaceId: "tenant-workspace-123",
  tenantId: "tenant-456",
});

// --- Pattern 5: Per-environment factory ---
function createClient(env: "development" | "staging" | "production") {
  const configs = {
    development: {
      baseUrl: "http://localhost:8080",
      timeout: 60_000, // Longer timeout for dev
    },
    staging: {
      baseUrl: "https://staging-api.edgequake.example.com",
      apiKey: process.env.STAGING_API_KEY,
      timeout: 30_000,
    },
    production: {
      baseUrl: "https://api.edgequake.example.com",
      apiKey: process.env.PRODUCTION_API_KEY,
      timeout: 15_000,
    },
  };

  return new EdgeQuake(configs[env]);
}

// --- Pattern 6: Health check before use ---
async function getHealthyClient(): Promise<EdgeQuake> {
  const client = new EdgeQuake({
    baseUrl: "http://localhost:8080",
  });

  const health = await client.health();
  if (health.status !== "healthy") {
    throw new Error(`Backend unhealthy: ${JSON.stringify(health)}`);
  }

  console.log("Connected to EdgeQuake:", {
    version: health.version,
    storage: health.storage_mode,
    provider: health.llm_provider_name,
  });

  return client;
}

async function main() {
  // Demo: use the environment factory
  const env =
    (process.env.NODE_ENV as "development" | "staging" | "production") ??
    "development";
  const client = createClient(env);
  console.log(`Created client for ${env} environment`);

  // Demo: health check pattern
  try {
    const healthyClient = await getHealthyClient();
    const docs = await healthyClient.documents.list({ page: 1, page_size: 5 });
    console.log(`Found ${docs.total} documents`);
  } catch (error) {
    console.log("Could not connect:", error);
  }
}

main().catch(console.error);
