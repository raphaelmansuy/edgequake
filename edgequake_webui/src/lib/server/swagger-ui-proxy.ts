/**
 * Dev-only Swagger UI reverse proxy (SPEC-035).
 *
 * Route handlers keep the browser URL on `/swagger-ui/…` so relative assets
 * resolve correctly; rewrites alone leave `/swagger-ui` without a slash and
 * break `./swagger-ui.css` resolution.
 */
import { NextRequest, NextResponse } from 'next/server';

import { resolveDevProxyBackend } from './dev-proxy-backend';

function resolveUpstreamUrl(
  backend: string,
  pathSegments: string[] | undefined,
  search: string,
): string {
  const subPath = pathSegments?.filter(Boolean).join('/') ?? '';
  const targetPath = subPath ? `/swagger-ui/${subPath}` : '/swagger-ui/';
  const targetUrl = new URL(`${backend}${targetPath}`);
  targetUrl.search = search;
  return targetUrl.toString();
}

function resolveRedirectUrl(backend: string, location: string): string {
  if (location.startsWith('http')) {
    return location;
  }
  return `${backend}${location.startsWith('/') ? location : `/${location}`}`;
}

export async function proxySwaggerUiRequest(
  request: NextRequest,
  pathSegments: string[] | undefined,
): Promise<NextResponse> {
  if (process.env.NODE_ENV !== 'development') {
    return NextResponse.json(
      { error: 'Swagger UI proxy is only available in development.' },
      { status: 404 },
    );
  }

  const backend = resolveDevProxyBackend().replace(/\/$/, '');
  const targetUrl = resolveUpstreamUrl(
    backend,
    pathSegments,
    request.nextUrl.search,
  );

  const headers: HeadersInit = {};
  const accept = request.headers.get('accept');
  if (accept) {
    headers.accept = accept;
  }

  let upstream = await fetch(targetUrl, {
    method: request.method,
    headers,
    redirect: 'manual',
  });

  if ([301, 302, 303, 307, 308].includes(upstream.status)) {
    const location = upstream.headers.get('location');
    if (location) {
      upstream = await fetch(resolveRedirectUrl(backend, location), {
        method: request.method,
        headers,
      });
    }
  }

  const responseHeaders = new Headers();
  const contentType = upstream.headers.get('content-type');
  if (contentType) {
    responseHeaders.set('content-type', contentType);
  }

  return new NextResponse(upstream.body, {
    status: upstream.status,
    headers: responseHeaders,
  });
}
