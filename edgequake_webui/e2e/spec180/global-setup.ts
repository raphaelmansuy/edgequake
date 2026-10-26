/** Refuse nonlocal or insecure stacks and wait for the actual API listener. */
export default async function setup() {
  const api = new URL(process.env.SPEC180_API_URL || 'http://127.0.0.1:18480/api/v1');
  if (!['127.0.0.1', 'localhost'].includes(api.hostname)) {
    throw new Error('SPEC-180 live fixtures require a disposable loopback API.');
  }
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(new URL('/health', api), { signal: AbortSignal.timeout(1500) });
      if (response.ok) {
        const health = await response.json();
        if (health.storage_mode !== 'postgresql' || !health.capabilities?.auth_enabled || health.capabilities?.dev_mode) {
          throw new Error('SPEC-180 requires PostgreSQL, enabled authentication, and disabled development mode.');
        }
        return;
      }
    } catch (error) {
      if (error instanceof Error && error.message.startsWith('SPEC-180 requires')) throw error;
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error('SPEC-180 API did not become ready within 45 seconds.');
}
