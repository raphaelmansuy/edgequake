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
import { imageAspectRatio } from './image-dimensions';

interface AuthenticatedMarkdownImageProps {
  src: string;
  alt?: string;
  title?: string;
  className?: string;
}

interface ShownImage {
  src: string;
  /** width / height from the file header or a decoded probe. */
  ratio: number | null;
}

function isMmAssetUrl(src: string): boolean {
  // Path splat: …/mm-assets/assets/page-0001.png
  // Id REST:    …/documents/{id}/assets/page-0001
  return (
    src.includes('/mm-assets/') ||
    /\/documents\/[^/]+\/assets\/[^/]+/.test(src)
  );
}

function ratioFromImage(img: HTMLImageElement): number | null {
  if (img.naturalWidth > 0 && img.naturalHeight > 0) {
    return img.naturalWidth / img.naturalHeight;
  }
  return null;
}

export function AuthenticatedMarkdownImage({
  src,
  alt,
  title,
  className,
}: AuthenticatedMarkdownImageProps) {
  const [shown, setShown] = useState<ShownImage | null>(null);

  useEffect(() => {
    let objectUrl: string | null = null;
    let cancelled = false;
    const ac = new AbortController();

    const publish = (nextSrc: string, ratio: number | null) => {
      if (!cancelled) setShown({ src: nextSrc, ratio });
    };

    if (!isMmAssetUrl(src)) {
      const probe = new Image();
      probe.onload = () => publish(src, ratioFromImage(probe));
      probe.onerror = () => publish(src, null);
      probe.src = src;
      return () => {
        cancelled = true;
      };
    }

    (async () => {
      // Always fetch with the full session headers (tenant/workspace + optional
      // bearer) — a plain <img src> would 404 under workspace scoping. This holds
      // in dev (auth off) too, where workspace scoping still applies.
      const headers = buildHeaders();
      headers.delete('Content-Type'); // GET with no body
      const res = await fetch(src, { headers, signal: ac.signal });
      if (!res.ok) {
        publish(src, null);
        return;
      }
      const blob = await res.blob();
      const bytes = new Uint8Array(await blob.arrayBuffer());
      const ratio = imageAspectRatio(bytes);
      objectUrl = URL.createObjectURL(blob);
      publish(objectUrl, ratio);
    })().catch(() => {
      if (!ac.signal.aborted) publish(src, null);
    });

    return () => {
      cancelled = true;
      ac.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [src]);

  const layoutAsset = layoutAssetStem(src);

  if (!shown) {
    return (
      <span
        data-layout-asset={layoutAsset}
        className={cn(
          'my-2 block min-h-32 w-full max-w-full rounded-md bg-muted/30 text-sm italic text-muted-foreground',
          className,
        )}
      >
        <span className="flex h-full min-h-32 items-center px-3">Loading image…</span>
      </span>
    );
  }

  return (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      src={shown.src}
      alt={alt ?? ''}
      title={title}
      width={shown.ratio ? 1000 : undefined}
      height={shown.ratio ? Math.round(1000 / shown.ratio) : undefined}
      data-layout-asset={layoutAsset}
      style={shown.ratio ? { aspectRatio: String(shown.ratio) } : undefined}
      className={cn(
        'my-2 h-auto w-full max-w-full rounded-md bg-muted/30',
        className,
        'data-[layout-asset-focused=true]:ring-2 data-[layout-asset-focused=true]:ring-primary',
      )}
      loading="lazy"
    />
  );
}
