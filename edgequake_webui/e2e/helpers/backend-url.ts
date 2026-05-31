/**
 * E2E backend URL — never hardcode localhost:8080 in specs.
 * Set via Makefile: EQ_BACKEND_URL / E2E_BACKEND_URL when ports are auto-selected.
 */
export const BACKEND_URL =
  process.env.EQ_BACKEND_URL ??
  process.env.E2E_BACKEND_URL ??
  process.env.SPEC013_BACKEND_URL ??
  "http://localhost:8080";

export const API_V1_URL = `${BACKEND_URL}/api/v1`;

/** True when backend health returns EdgeQuake JSON (not a foreign app on :8080). */
export async function isEdgequakeBackendHealthy(
  request: { get: (url: string) => Promise<{ ok: () => boolean; json: () => Promise<unknown> }> },
): Promise<boolean> {
  try {
    const response = await request.get(`${BACKEND_URL}/health`);
    if (!response.ok()) return false;
    const body = (await response.json()) as { status?: string; storage_mode?: string };
    return body.status === "healthy" && typeof body.storage_mode === "string";
  } catch {
    return false;
  }
}
