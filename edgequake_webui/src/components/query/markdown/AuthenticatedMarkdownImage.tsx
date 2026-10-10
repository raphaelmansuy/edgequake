/**
 * Load markdown images that require API auth + tenant/workspace headers.
 *
 * Browser `<img src>` cannot send `Authorization`, `X-Tenant-ID`, or
 * `X-Workspace-ID`. mm-asset serving is workspace-scoped (SPEC-091 SSOT): a
 * headerless `<img src>` defaults to the default workspace on the backend and
 * 404s the document. For mm-asset URLs we therefore fetch with the full session
 * headers (`buildHeaders` — tenant/workspace + optional bearer) and display a
 * blob URL (DRY with PDF/WS auth patterns).
 */
'use client';

import { layoutAssetStem } from '@/components/documents/layout-asset';
import { buildHeaders } from '@/lib/api/client';
import { cn } from '@/lib/utils';
import { useEffect, useState } from 'react';

interface AuthenticatedMarkdownImageProps {
  src: string;
  alt?: string;
  title?: string;
  className?: string;
}

function isMmAssetUrl(src: string): boolean {
  // Path splat: …/mm-assets/assets/page-0001.png
  // Id REST:    …/documents/{id}/assets/page-0001
  return (
    src.includes('/mm-assets/') ||
    /\/documents\/[^/]+\/assets\/[^/]+/.test(src)
  );
}

export function AuthenticatedMarkdownImage({
  src,
  alt,
  title,
  className,
}: AuthenticatedMarkdownImageProps) {
  const [resolvedSrc, setResolvedSrc] = useState<string | null>(
    isMmAssetUrl(src) ? null : src,
  );

  useEffect(() => {
    if (!isMmAssetUrl(src)) return;

    let objectUrl: string | null = null;
    const ac = new AbortController();

    (async () => {
      // Always fetch with the full session headers (tenant/workspace + optional
      // bearer) — a plain <img src> would 404 under workspace scoping. This holds
      // in dev (auth off) too, where workspace scoping still applies.
      const headers = buildHeaders();
      headers.delete('Content-Type'); // GET with no body
      const res = await fetch(src, { headers, signal: ac.signal });
      if (!res.ok) {
        setResolvedSrc(src);
        return;
      }
      const blob = await res.blob();
      objectUrl = URL.createObjectURL(blob);
      setResolvedSrc(objectUrl);
    })().catch(() => {
      if (!ac.signal.aborted) {
        setResolvedSrc(src);
      }
    });

    return () => {
      ac.abort();
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [src]);

  const layoutAsset = layoutAssetStem(src);
  // Non-asset URLs render directly; asset URLs wait for the authenticated blob.
  const shownSrc = isMmAssetUrl(src) ? resolvedSrc : src;

  return (
    <span className="my-2 block aspect-video w-full max-w-full overflow-hidden rounded-md bg-muted/30">
      {shownSrc ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={shownSrc}
          alt={alt ?? ''}
          title={title}
          data-layout-asset={layoutAsset}
          className={cn(
            'h-full w-full object-contain',
            className,
            'data-[layout-asset-focused=true]:ring-2 data-[layout-asset-focused=true]:ring-primary',
          )}
          loading="lazy"
        />
      ) : (
        <span
          data-layout-asset={layoutAsset}
          className="flex h-full items-center px-3 text-sm italic text-muted-foreground"
        >
          Loading image…
        </span>
      )}
    </span>
  );
}
