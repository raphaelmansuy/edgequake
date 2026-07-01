import { NextRequest } from 'next/server';

import { proxySwaggerUiRequest } from '@/lib/server/swagger-ui-proxy';

export const dynamic = 'force-dynamic';

async function handle(
  request: NextRequest,
  context: { params: Promise<{ path?: string[] }> },
) {
  const { path } = await context.params;
  return proxySwaggerUiRequest(request, path);
}

export const GET = handle;
export const HEAD = handle;
