/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { serverRootClient } from "@/lib/api/client";
import type { HealthResponse } from "@/types";

export async function checkHealth(): Promise<HealthResponse> {
  return serverRootClient<HealthResponse>("/health", { silent: true });
}

export async function checkReady(): Promise<{ status: string }> {
  return serverRootClient<{ status: string }>("/ready", { silent: true });
}
