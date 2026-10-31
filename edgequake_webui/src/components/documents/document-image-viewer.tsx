'use client';

/**
 * SPEC-180 — the document detail view for an image.
 *
 * WHY: an image document stores its file name as `content` (the pixels are the payload, and
 * the worker needs a non-empty body). The generic renderer therefore painted the *filename* in
 * a text card — a `.png` opened to the word "Screenshot_…png". The picture itself is served by
 * `GET /documents/{id}/download/original`, which needs tenant/workspace/bearer headers, so it
 * cannot go in a bare `<img src>`.
 *
 * This view fetches the bytes with those headers, shows them on a neutral stage that fits both
 * orientations, and offers the actions an image actually needs: zoom, fit, download, open.
 */

import { useAuthedImageUrl } from '@/hooks/use-authed-image-url';
import { getDocumentOriginalDownloadUrl } from '@/lib/api/edgequake';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import {
  AlertCircle,
  Download,
  ExternalLink,
  Image as ImageIcon,
  Loader2,
  Maximize2,
  Minus,
  Plus,
  RotateCcw,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface DocumentImageViewerProps {
  document: Pick<Document, 'id' | 'title' | 'file_name' | 'mime_type' | 'file_size'>;
  className?: string;
}

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4;
const ZOOM_STEP = 0.25;

function formatBytes(bytes: number | undefined): string | null {
  if (!bytes || bytes <= 0) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function DocumentImageViewer({ document, className }: DocumentImageViewerProps) {
  const { t } = useTranslation();
  const src = getDocumentOriginalDownloadUrl(document.id);
  const image = useAuthedImageUrl(src);
  const [zoom, setZoom] = useState(1);
  const [natural, setNatural] = useState<{ width: number; height: number } | null>(null);
  const stageRef = useRef<HTMLDivElement>(null);

  const name = document.file_name || document.title || document.id;

  const reset = useCallback(() => setZoom(1), []);
  const zoomBy = useCallback(
    (delta: number) =>
      setZoom((z) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Number((z + delta).toFixed(2))))),
    [],
  );

  // Keyboard: +/- zoom, 0 to fit. Only while the stage has focus, so the page keeps its own
  // shortcuts everywhere else.
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === '+' || event.key === '=') {
      event.preventDefault();
      zoomBy(ZOOM_STEP);
    } else if (event.key === '-' || event.key === '_') {
      event.preventDefault();
      zoomBy(-ZOOM_STEP);
    } else if (event.key === '0') {
      event.preventDefault();
      reset();
    }
  };

  // Ctrl/Cmd + wheel zooms the picture, the way every image viewer behaves. Registered
  // non-passively so the browser's page zoom does not fire instead.
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage || image.status !== 'ready') return;
    const handler = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      zoomBy(event.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP);
    };
    stage.addEventListener('wheel', handler, { passive: false });
    return () => stage.removeEventListener('wheel', handler);
  }, [image.status, zoomBy]);

  const meta = [natural ? `${natural.width} × ${natural.height}` : null, formatBytes(document.file_size)]
    .filter(Boolean)
    .join(' · ');

  return (
    <div className={cn('flex h-full min-h-0 flex-col', className)} data-testid="document-image-viewer">
      {/* Toolbar: what it is, how big, and what you can do with it. */}
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b bg-muted/30 px-3 py-1.5">
        <span className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
          <ImageIcon aria-hidden className="h-3.5 w-3.5 shrink-0" />
          <span className="truncate" title={name}>
            {name}
          </span>
        </span>
        {meta ? (
          <span className="text-xs tabular-nums text-muted-foreground/80" data-testid="document-image-meta">
            {meta}
          </span>
        ) : null}

        <div className="ml-auto flex items-center gap-1">
          <button
            type="button"
            onClick={() => zoomBy(-ZOOM_STEP)}
            disabled={image.status !== 'ready' || zoom <= ZOOM_MIN}
            className="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40"
            aria-label={t('documents.image.zoomOut', 'Zoom out')}
            data-testid="document-image-zoom-out"
          >
            <Minus className="h-3.5 w-3.5" />
          </button>
          <span
            className="min-w-[3.25rem] text-center text-xs tabular-nums text-muted-foreground"
            data-testid="document-image-zoom-level"
            aria-live="polite"
          >
            {Math.round(zoom * 100)}%
          </span>
          <button
            type="button"
            onClick={() => zoomBy(ZOOM_STEP)}
            disabled={image.status !== 'ready' || zoom >= ZOOM_MAX}
            className="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40"
            aria-label={t('documents.image.zoomIn', 'Zoom in')}
            data-testid="document-image-zoom-in"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={reset}
            disabled={image.status !== 'ready' || zoom === 1}
            className="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40"
            aria-label={t('documents.image.fit', 'Fit to window')}
            title={t('documents.image.fit', 'Fit to window')}
            data-testid="document-image-fit"
          >
            <RotateCcw className="h-3.5 w-3.5" />
          </button>
          <span aria-hidden className="mx-0.5 h-4 w-px bg-border" />
          <a
            href={src}
            download={name}
            className="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
            aria-label={t('documents.image.download', 'Download original')}
            title={t('documents.image.download', 'Download original')}
            data-testid="document-image-download"
          >
            <Download className="h-3.5 w-3.5" />
          </a>
          {image.status === 'ready' ? (
            <a
              href={image.url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
              aria-label={t('documents.image.open', 'Open full size')}
              title={t('documents.image.open', 'Open full size')}
              data-testid="document-image-open"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </a>
          ) : null}
        </div>
      </div>

      {/* Stage: a checkerboard-free neutral mat, so a transparent PNG still reads. */}
      <div
        ref={stageRef}
        role="group"
        tabIndex={0}
        onKeyDown={onKeyDown}
        aria-label={t('documents.image.stage', 'Image preview. Use plus and minus to zoom.')}
        // A soft neutral mat, not a transparency checkerboard: the checker reads as "no
        // image" (the same signal as a broken <img>) and fought the picture's own background.
        className="min-h-0 flex-1 overflow-auto bg-muted/30 p-4 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        data-testid="document-image-stage"
      >
        {image.status === 'loading' ? (
          <div className="flex h-full items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" aria-hidden />
            <span className="sr-only">{t('documents.image.loading', 'Loading image…')}</span>
          </div>
        ) : image.status === 'failed' ? (
          <div
            className="flex h-full flex-col items-center justify-center gap-2 text-center"
            data-testid="document-image-failed"
            role="alert"
          >
            <AlertCircle className="h-8 w-8 text-muted-foreground" aria-hidden />
            <p className="text-sm font-medium">{t('documents.image.failed', 'The image could not be loaded')}</p>
            <p className="max-w-sm text-xs text-muted-foreground">
              {t(
                'documents.image.failedHint',
                'It may still be processing, or the original is no longer stored. Try again, or reprocess the document.',
              )}
            </p>
            <a
              href={src}
              className="text-xs font-medium text-primary underline-offset-2 hover:underline"
              target="_blank"
              rel="noreferrer"
            >
              {t('documents.image.open', 'Open full size')}
            </a>
          </div>
        ) : (
          <div className="flex min-h-full items-center justify-center">
            {/* eslint-disable-next-line @next/next/no-img-element -- blob URL from an authed fetch */}
            <img
              src={image.url}
              alt={document.title || name}
              onLoad={(event) =>
                setNatural({
                  width: event.currentTarget.naturalWidth,
                  height: event.currentTarget.naturalHeight,
                })
              }
              style={{
                // Fit is the default: `max-*` keeps a wide picture inside the pane; zoom
                // multiplies from there and lets the stage scroll.
                maxWidth: zoom === 1 ? '100%' : undefined,
                maxHeight: zoom === 1 ? '100%' : undefined,
                width: zoom === 1 ? undefined : `${zoom * 100}%`,
                height: 'auto',
              }}
              className={cn(
                'rounded-md shadow-sm ring-1 ring-black/5 dark:ring-white/10',
                zoom !== 1 && 'max-w-none',
              )}
              data-testid="document-image"
            />
          </div>
        )}
      </div>
    </div>
  );
}

/** True when a document is an image the viewer should render instead of text. */
export function isImageDocument(
  document: Pick<Document, 'source_type' | 'mime_type' | 'file_name' | 'title'>,
): boolean {
  if (document.source_type === 'image') return true;
  const mime = (document.mime_type || '').toLowerCase();
  if (mime.startsWith('image/')) return true;
  const name = (document.file_name || document.title || '').toLowerCase();
  return /\.(png|jpe?g|webp|gif|bmp|tiff?|avif)$/.test(name);
}

export default DocumentImageViewer;
