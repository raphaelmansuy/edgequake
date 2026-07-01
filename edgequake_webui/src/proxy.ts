import { NextRequest, NextResponse } from 'next/server';

/**
 * Swagger UI HTML uses relative asset URLs (`./swagger-ui.css`). Without a
 * trailing slash the browser resolves them to `/swagger-ui.css` and the page
 * stays blank. Redirect once to the canonical path before the route handler runs.
 */
export function proxy(request: NextRequest) {
  if (request.nextUrl.pathname === '/swagger-ui') {
    // Plain URL — NextURL strips trailing slashes when skipTrailingSlashRedirect is set.
    return NextResponse.redirect(new URL('/swagger-ui/', request.url), 307);
  }

  return NextResponse.next();
}

export const config = {
  matcher: '/swagger-ui',
};
