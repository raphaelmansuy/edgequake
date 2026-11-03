'use client';

/**
 * SPEC-180 — the pictures an answer found, as a thumbnail grid.
 *
 * Shown above the source chips whenever the answer cites image documents (search by image,
 * or any question an image answers). Each tile opens the source; the `[n]` is the citation
 * number the answer uses.
 */
import { Button } from '@/components/ui/button';
import { useAuthedImageUrl } from '@/hooks/use-authed-image-url';
import { useOpenSource } from '@/hooks/use-open-source';
import { getDocumentOriginalDownloadUrl } from '@/lib/api/edgequake/documents';
import { formatMatchPercent } from '@/lib/citations/citation-href';
import {
  balancedColumns,
  imageMatches,
  relevantImageMatches,
  type ImageMatch,
} from '@/lib/citations/image-matches';
import { locationFromChunk } from '@/lib/query/companion-pane';
import type { QueryContext } from '@/types';
import { ImageOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/** Tiles shown before "+N more". Two full rows at the widest grid. */
const VISIBLE_TILES = 12;
/**
 * Most tiles per row at the phone, tablet and desktop widths.
 * WHY unchanged: 5 matches read as 3 + 2 on a phone and a single row on desktop
 * (T-180-ui-image-grid). Widening the tiles changed that balance for no gain.
 */
const MAX_COLUMNS = { base: 3, sm: 4, lg: 6 } as const;
/** A tile never grows past this, so one match is a thumbnail and not a banner. */
const TILE_MAX = '9.5rem';

const GRID_CLASS =
  'grid justify-start gap-2 ' +
  'grid-cols-[repeat(var(--cols-base),minmax(0,var(--tile-max)))] ' +
  'sm:grid-cols-[repeat(var(--cols-sm),minmax(0,var(--tile-max)))] ' +
  'lg:grid-cols-[repeat(var(--cols-lg),minmax(0,var(--tile-max)))]';

function gridStyle(count: number): React.CSSProperties {
  return {
    '--tile-max': TILE_MAX,
    '--cols-base': balancedColumns(count, MAX_COLUMNS.base),
    '--cols-sm': balancedColumns(count, MAX_COLUMNS.sm),
    '--cols-lg': balancedColumns(count, MAX_COLUMNS.lg),
  } as React.CSSProperties;
}

function Thumbnail({ documentId, alt }: { documentId: string; alt: string }) {
  const { t } = useTranslation();
  const image = useAuthedImageUrl(getDocumentOriginalDownloadUrl(documentId));
  if (image.status === 'ready') {
    return (
      // eslint-disable-next-line @next/next/no-img-element -- a blob: URL cannot use next/image
      <img
        src={image.url}
        alt={alt}
        // `contain` on a neutral mat: the evidence is the whole picture. `cover` cropped
        // screenshots and receipts to an unrecognisable centre.
        className="h-full w-full object-contain transition-transform duration-200 ease-out group-hover/tile:scale-[1.02]"
      />
    );
  }
  if (image.status === 'failed') {
    return (
      <span className="flex h-full w-full flex-col items-center justify-center gap-1 text-muted-foreground">
        <ImageOff className="h-5 w-5" aria-hidden />
        <span className="text-[10px] uppercase tracking-wide">
          {t('query.imageMatches.unavailable', 'No preview')}
        </span>
      </span>
    );
  }
  return <span className="block h-full w-full animate-pulse bg-muted" aria-hidden />;
}

function Tile({ match, onOpen }: { match: ImageMatch; onOpen: (match: ImageMatch) => void }) {
  const { t } = useTranslation();
  const percent = formatMatchPercent(match.chunk.score);
  return (
    <li className="min-w-0">
      <button
        type="button"
        onClick={() => onOpen(match)}
        className="group/tile block w-full min-w-0 overflow-hidden rounded-xl border border-border/60 bg-card text-left transition-colors hover:border-primary/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
        aria-label={t('query.imageMatches.open', 'Open image {{index}}: {{title}}', {
          index: match.index,
          title: match.title,
        })}
        title={match.title}
        data-testid="query-image-match"
      >
        <span className="relative block aspect-[4/3] overflow-hidden bg-muted/30">
          <Thumbnail documentId={match.chunk.document_id} alt={match.title} />
          {/* Citation number stays on the picture: it is what the answer refers to. */}
          <span className="absolute left-1.5 top-1.5 rounded-md bg-background/90 px-1.5 py-0.5 text-[11px] font-semibold tabular-nums shadow-sm">
            [{match.index}]
          </span>
        </span>
        <span className="flex items-baseline justify-between gap-1.5 px-2 py-1.5">
          <span className="min-w-0 truncate text-xs text-foreground/80" title={match.title}>
            {match.title}
          </span>
          {/* Relatedness reads as a number, not an unexplained floating badge. */}
          {percent === null ? null : (
            <span
              className="shrink-0 text-[11px] tabular-nums text-muted-foreground"
              title={t('query.imageMatches.relevance', 'Match strength')}
            >
              {percent}%
            </span>
          )}
        </span>
    </button>
    </li>
  );
}

export function ImageMatchesGrid({
  chunks,
  onOpenSources,
}: {
  chunks: QueryContext['chunks'] | undefined;
  /** "+N more" reveals the full sources panel. */
  onOpenSources?: () => void;
}) {
  const { t } = useTranslation();
  const openSource = useOpenSource();
  // SPEC-183: only pictures that actually matched the question. Retrieval always returns
  // `top_k`, so "what came back" is not "what is relevant" — a refund question used to display
  // all twenty photographs. `relevantImageMatches` drops any tile whose measured similarity is
  // below the floor; a payload without similarities keeps its tiles, so nothing regresses.
  const all = relevantImageMatches(chunks);
  if (all.length === 0) return null;
  const shown = all.slice(0, VISIBLE_TILES);
  const hidden = all.length - shown.length;
  // How many were retrieved but judged not relevant enough to show. Used only to keep the
  // heading honest when the filter removed something.
  const suppressed = imageMatches(chunks).length - all.length;

  return (
    <section className="mb-3 space-y-2" data-testid="query-image-matches" aria-label={t('query.imageMatches.title', 'Matching images')}>
      <h3 className="text-xs font-medium text-muted-foreground">
        {t('query.imageMatches.title', 'Matching images')} · {all.length}
        {suppressed > 0 ? (
          <span
            className="ml-1.5 font-normal text-muted-foreground/70"
            data-testid="query-image-matches-suppressed"
            title={t(
              'query.imageMatches.suppressedHint',
              '{{count}} more pictures were retrieved but did not match closely enough to show.',
              { count: suppressed },
            )}
          >
            {t('query.imageMatches.suppressed', '+{{count}} weak', { count: suppressed })}
          </span>
        ) : null}
      </h3>
      <ul className={GRID_CLASS} style={gridStyle(shown.length)}>
        {shown.map((match) => (
          <Tile
            key={match.chunk.document_id}
            match={match}
            onOpen={(m) => openSource(locationFromChunk(m.chunk))}
          />
        ))}
      </ul>
      {hidden > 0 ? (
        <Button type="button" variant="ghost" size="sm" className="h-7 px-2 text-xs" onClick={onOpenSources}>
          {t('query.imageMatches.more', '+{{count}} more', { count: hidden })}
        </Button>
      ) : null}
    </section>
  );
}
