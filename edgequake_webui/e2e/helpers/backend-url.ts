/**
 * E2E backend URL — never hardcode localhost:8080 in specs.
 * Set via Makefile: EQ_BACKEND_URL / E2E_BACKEND_URL when ports are auto-selected.
 */
export const BACKEND_URL =
  process.env.EQ_BACKEND_URL ??
  process.env.E2E_BACKEND_URL ??
  process.env.EDGEQUAKE_API_URL ??
  process.env.SPEC013_BACKEND_URL ??
  "http://localhost:8080";

export const API_V1_URL = `${BACKEND_URL}/api/v1`;

/** True when backend health returns EdgeQuake JSON (not a foreign app on :8080). */
export async function isEdgequakeBackendHealthy(
  request: { get: (url: string) => Promise<{ ok: () => boolean; json: () => Promise<unknown> }> },
): Promise<boolean> {
  for (const path of ["/health", "/api/v1/health"]) {
    try {
      const response = await request.get(`${BACKEND_URL}${path}`);
      if (!response.ok()) continue;
      const body = (await response.json()) as { status?: string; storage_mode?: string };
      if (body.status === "healthy" && typeof body.storage_mode === "string") {
        return true;
      }
    } catch {
      /* try next path */
    }
  }
  return false;
}

/** Poll until backend is ready (cold start after make dev-bg). */
export async function waitForBackendInGlobalSetup(
  request: { get: (url: string) => Promise<{ ok: () => boolean; json: () => Promise<unknown> }> },
  maxAttempts = 45,
): Promise<boolean> {
  for (let i = 0; i < maxAttempts; i++) {
    if (await isEdgequakeBackendHealthy(request)) return true;
    await new Promise((r) => setTimeout(r, 2000));
  }
  return false;
}
