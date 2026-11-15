/**
 * @fileoverview Document hierarchy tree showing Document → Chunks → Entities
 *
 * WHY: The existing LineageTree shows pipeline steps (upload, extract, map, index)
 * but not the actual data hierarchy. This component shows the real structure:
 * which chunks were created, which entities were extracted from each chunk,
 * enabling source traceability.
 *
 * @implements FEAT1088 - Document hierarchy tree visualization
 * @implements F8 - PDF → Document → Chunk → Entity chain traceable
 *
 * @see UC1519 - User views document-to-entity hierarchy
 * @enforces BR1088 - Collapsible tree with entity counts per chunk
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useDocumentFullLineage } from '@/hooks/use-lineage';
import { useQueryHandoff } from '@/hooks/use-query-handoff';
import { buildDocumentPageUrl, formatChunkPageBadge } from '@/lib/utils/document-url';
import { cn } from '@/lib/utils';
import type { EntityLineage } from '@/types/lineage';
import {
  ChevronDown,
  ChevronRight,
  ExternalLink,
  FileText,
  Layers,
  Loader2,
  MessageSquareText,
  Tag,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

/**
 * Chunk shape from the /documents/:id/lineage endpoint.
 * WHY: The full-lineage endpoint returns chunks with entity_ids (not token_count).
 */
interface FullLineageChunk {
  chunk_id: string;
  chunk_index: number;
  start_line?: number;
  end_line?: number;
  start_offset?: number;
  end_offset?: number;
  entity_ids?: string[];
  extraction_metadata?: Record<string, unknown>;
  relationship_ids?: string[];
  /** PDF page number (1-indexed). Present only for PDFs. SPEC-033 / SPEC-135. */
  page_start?: number;
  /** Last PDF page in the chunk (`≥ page_start`). SPEC-135 span. */
  page_end?: number;
}

/**
 * Entity shape from the /documents/:id/lineage endpoint.
 * WHY: Entities are stored as a dict keyed by entity_id in the KV lineage,
 * not as an array like the graph-based endpoint.
 */
interface FullLineageEntity {
  entity_id: string;
  entity_name: string;
  extraction_count: number;
  sources?: Array<{ chunk_ids?: string[] }>;
  description_history?: Array<{ description?: string }>;
}

interface DocumentHierarchyTreeProps {
  documentId: string;
  documentName?: string;
  /**
   * Whether the document is still being processed.
   *
   * WHY: the lineage KV record is only written at pipeline finalize, so the
   * lineage request 404s for the whole ingestion window. When this is true the
   * tree keeps polling so it populates as soon as the data lands instead of
   * staying stuck on "Hierarchy data not available".
   */
  isDocumentProcessing?: boolean;
  /** Called when a chunk is clicked; provides line range and page for navigation. */
  onChunkSelect?: (chunkId: string, startLine?: number, endLine?: number, page?: number) => void;
  /**
   * Called once after chunk data loads when selectedChunkId is already set
   * (e.g. arriving from a deep-link URL). Fires with the resolved line range
   * so ContentRenderer can scroll to and highlight the chunk.
   * Unlike onChunkSelect this does NOT toggle selection.
   */
  onChunkResolved?: (chunkId: string, startLine?: number, endLine?: number, page?: number) => void;
  /** ID of the currently selected chunk (controls visual highlight in tree). */
  selectedChunkId?: string;
}

export function DocumentHierarchyTree({
  documentId,
  documentName,
  isDocumentProcessing = false,
  onChunkSelect,
  onChunkResolved,
  selectedChunkId,
}: DocumentHierarchyTreeProps) {
  // WHY: useDocumentFullLineage calls /documents/:id/lineage which returns
  // persisted KV lineage with actual chunks and entity data.
  // The old useDocumentLineage (/lineage/documents/:id) returned graph-based
  // data without chunk details, causing "0 chunks" display.
  const { data: fullLineage, isLoading, error } = useDocumentFullLineage(
    documentId,
    { isDocumentProcessing },
  );

  // WHY: the backend now separates "not written yet" from "this pipeline writes no
  // lineage record". Surfacing that message turns a dead end into an explanation.
  const errorMessage = useMemo(() => {
    if (!error) return null;
    const message = (error as { message?: string }).message;
    return message?.trim() ? message : null;
  }, [error]);

  // WHY: Transform the raw response into normalized arrays.
  // The full-lineage endpoint returns entities as a dict (keyed by entity_id)
  // and chunks as objects with entity_ids arrays.
  // useMemo MUST be called unconditionally before any early returns
  // to satisfy React Rules of Hooks (same order on every render).
  const { chunks, entitiesByChunk, totalEntities, docName } = useMemo(() => {
    const lineage = fullLineage?.lineage as Record<string, unknown> | undefined;
    if (!lineage) {
      return { chunks: [] as FullLineageChunk[], entitiesByChunk: new Map<string, EntityLineage[]>(), totalEntities: 0, docName: '' };
    }

    // Extract chunks array
    const rawChunks = (lineage.chunks ?? []) as FullLineageChunk[];

    // Extract entities dict → normalize to EntityLineage[]
    const rawEntities = (lineage.entities ?? {}) as Record<string, FullLineageEntity>;
    const entityArray: EntityLineage[] = Object.values(rawEntities).map((e) => ({
      id: e.entity_id,
      name: e.entity_name,
      entity_type: '', // Not stored in KV lineage
      source_chunks: (e.sources ?? []).flatMap((s) => s.chunk_ids ?? []),
      extraction_count: e.extraction_count ?? 1,
    }));

    // Build chunk_id → entities lookup.
    // WHY: Deduplicate per-chunk to avoid duplicate React keys.
    // source_chunks can contain repeated chunkId entries (backend bug or
    // extraction retry), which would generate sibling nodes with identical keys.
    const byChunk = new Map<string, EntityLineage[]>();
    for (const entity of entityArray) {
      const seenChunks = new Set<string>();
      for (const chunkId of entity.source_chunks) {
        if (seenChunks.has(chunkId)) continue; // skip duplicate chunk references
        seenChunks.add(chunkId);
        const list = byChunk.get(chunkId) ?? [];
        list.push(entity);
        byChunk.set(chunkId, list);
      }
    }

    return {
      chunks: rawChunks,
      entitiesByChunk: byChunk,
      totalEntities: entityArray.length,
      docName: (lineage.document_name as string) ?? '',
    };
  }, [fullLineage?.lineage]);

  // When chunk data finishes loading and a chunk is already selected (via URL
  // deep-link), resolve its line range and report it back so the content area
  // can scroll to and highlight the correct passage.
  // WHY: onChunkSelect (click handler) sets the line range for user interactions,
  // but when arriving from a citation URL only the chunk ID is in the URL — the
  // line range must be looked up from the loaded lineage data.
  useEffect(() => {
    if (!selectedChunkId || chunks.length === 0 || !onChunkResolved) return;
    const found = chunks.find((c) => c.chunk_id === selectedChunkId);
    if (found) {
      onChunkResolved(found.chunk_id, found.start_line, found.end_line, found.page_start);
    }
  }, [chunks, selectedChunkId, onChunkResolved]);

  useEffect(() => {
    if (!selectedChunkId) return;
    const frame = window.requestAnimationFrame(() => {
      const row = document.querySelector(
        `[data-testid="hierarchy-chunk-row"][data-chunk-id="${CSS.escape(selectedChunkId)}"]`,
      );
      row?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [chunks, selectedChunkId]);

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground p-2">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        Loading hierarchy...
      </div>
    );
  }

  if (error || !fullLineage) {
    // WHY distinguish these: lineage is only persisted at pipeline finalize, so
    // "still processing" is a normal pending state, not a failure. Showing the
    // same permanent-looking message for both made a healthy ingestion look broken.
    if (isDocumentProcessing) {
      return (
        <div className="flex items-center gap-2 text-sm text-muted-foreground p-2">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          Available when processing completes
        </div>
      );
    }
    return (
      <div className="p-2 space-y-1">
        <p className="text-xs text-muted-foreground">
          Hierarchy data not available
        </p>
        {/* WHY show the reason: a 404 here has two very different causes (still
            pending vs. a pipeline that writes no lineage record), and the server
            now distinguishes them. Hiding it left users with no next step. */}
        {errorMessage && (
          <p className="text-xs text-muted-foreground/70">{errorMessage}</p>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {/* Document root node */}
      <TreeNode
        icon={<FileText className="h-3.5 w-3.5" />}
        label={documentName ?? docName ?? documentId.slice(0, 8)}
        badge={`${chunks.length} chunks • ${totalEntities} entities`}
        defaultOpen
        depth={0}
      >
        {chunks.length === 0 ? (
          <p className="text-xs text-muted-foreground pl-6 py-1">
            No chunks extracted yet
          </p>
        ) : (
          <ChunkList
            chunks={chunks}
            entitiesByChunk={entitiesByChunk}
            documentId={documentId}
            documentTitle={documentName ?? docName ?? documentId.slice(0, 8)}
            selectedChunkId={selectedChunkId}
            onChunkSelect={onChunkSelect}
          />
        )}
      </TreeNode>
    </div>
  );
}

// ============================================================================
// Page-aware chunk list (SPEC-033)
// ============================================================================

/**
 * Group chunks by page number.
 * Returns null when no chunk has a valid page_start (non-PDF fallback).
 *
 * @implements SPEC-033 FR-003 — page-grouped Data Hierarchy
 */
function groupChunksByPage(
  chunks: FullLineageChunk[],
): Map<number, FullLineageChunk[]> | null {
  const hasPages = chunks.some(
    (c) => c.page_start !== undefined && c.page_start > 0,
  );
  if (!hasPages) return null;

  const map = new Map<number, FullLineageChunk[]>();
  for (const chunk of chunks) {
    const page =
      chunk.page_start !== undefined && chunk.page_start > 0
        ? chunk.page_start
        : 0; // 0 = "no page" bucket
    const list = map.get(page) ?? [];
    list.push(chunk);
    map.set(page, list);
  }
  return map;
}

interface ChunkListProps {
  chunks: FullLineageChunk[];
  entitiesByChunk: Map<string, EntityLineage[]>;
  documentId: string;
  documentTitle: string;
  selectedChunkId?: string;
  onChunkSelect?: (chunkId: string, startLine?: number, endLine?: number, page?: number) => void;
}

function ChunkList({
  chunks,
  entitiesByChunk,
  documentId,
  documentTitle,
  selectedChunkId,
  onChunkSelect,
}: ChunkListProps) {
  const grouped = groupChunksByPage(chunks);

  if (grouped) {
    // Page-grouped mode (FR-003)
    const entries = [...grouped.entries()].sort(([a], [b]) => a - b);
    return (
      <>
        {entries.map(([page, pageChunks]) =>
          page === 0 ? (
            // "No page" bucket — render flat without page header
            pageChunks.map((chunk) => (
              <ChunkTreeNode
                key={chunk.chunk_id}
                chunk={chunk}
                entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
                documentId={documentId}
                documentTitle={documentTitle}
                depth={1}
                isSelected={selectedChunkId === chunk.chunk_id}
                onSelect={onChunkSelect}
              />
            ))
          ) : (
            <PageGroupNode
              key={`page-${page}`}
              page={page}
              chunks={pageChunks}
              entitiesByChunk={entitiesByChunk}
              documentId={documentId}
              documentTitle={documentTitle}
              selectedChunkId={selectedChunkId}
              onSelect={onChunkSelect}
            />
          ),
        )}
      </>
    );
  }

  // Flat mode — non-PDF or no page data (FR-010)
  return (
    <>
      {chunks.map((chunk) => (
        <ChunkTreeNode
          key={chunk.chunk_id}
          chunk={chunk}
          entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
          documentId={documentId}
          documentTitle={documentTitle}
          depth={1}
          isSelected={selectedChunkId === chunk.chunk_id}
          onSelect={onChunkSelect}
        />
      ))}
    </>
  );
}

// ============================================================================
// Page group node (SPEC-033)
// ============================================================================

interface PageGroupNodeProps {
  page: number;
  chunks: FullLineageChunk[];
  entitiesByChunk: Map<string, EntityLineage[]>;
  documentId: string;
  documentTitle: string;
  selectedChunkId?: string;
  onSelect?: (chunkId: string, startLine?: number, endLine?: number, page?: number) => void;
}

/**
 * Collapsible group header for a PDF page.
 * Shows "Page N" with chunk + entity counts and a deeplink badge.
 *
 * @implements SPEC-033 FR-003, FR-004, FR-007
 */
function PageGroupNode({
  page,
  chunks,
  entitiesByChunk,
  documentId,
  documentTitle,
  selectedChunkId,
  onSelect,
}: PageGroupNodeProps) {
  const containsSelected = chunks.some((c) => c.chunk_id === selectedChunkId);
  // First 3 pages open by default; the page that owns the deeplinked chunk
  // must also open so the selected row is not hidden (SPEC-135 page=4+).
  const [isOpen, setIsOpen] = useState(page <= 3 || containsSelected);

  useEffect(() => {
    if (containsSelected) setIsOpen(true);
  }, [containsSelected]);

  const entityCount = chunks.reduce(
    (acc, c) => acc + (entitiesByChunk.get(c.chunk_id)?.length ?? 0),
    0,
  );

  const pageUrl = buildDocumentPageUrl(documentId, undefined, page);

  const toggle = useCallback(() => setIsOpen((o) => !o), []);
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    },
    [toggle],
  );

  return (
    <div className="ml-0">
      {/* Page header row */}
      <div
        role="button"
        tabIndex={0}
        aria-expanded={isOpen}
        aria-label={`Page ${page}: ${chunks.length} chunk${chunks.length !== 1 ? 's' : ''}`}
        data-testid="page-group"
        data-page={page}
        className={cn(
          'group flex items-center gap-1.5 w-full text-left py-1 px-2 rounded',
          'hover:bg-muted/40 transition-colors cursor-pointer select-none',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50',
        )}
        style={{ paddingLeft: '16px' }}
        onClick={toggle}
        onKeyDown={handleKeyDown}
      >
        {isOpen ? (
          <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        )}
        {/* Page icon — contrasting colour to distinguish from chunk icon */}
        <Layers className="h-3 w-3 shrink-0 text-primary/70" />
        <span className="text-xs font-semibold text-foreground/90">
          Page {page}
        </span>
        {/* Counts — right-aligned */}
        <span className="text-xs text-muted-foreground ml-auto shrink-0 pr-1">
          {chunks.length} chunk{chunks.length !== 1 ? 's' : ''} · {entityCount} ent
        </span>
        {/* Page deeplink badge — appears on hover; stops propagation so only the
            badge navigates, not the group toggle. FR-007 */}
        <Link
          href={pageUrl}
          className={cn(
            'inline-flex items-center gap-0.5 text-xs font-medium',
            'text-primary hover:underline shrink-0',
            'opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity',
            'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50',
            'rounded-sm px-1 py-0.5',
          )}
          aria-label={`Go to page ${page}`}
          title={`Open PDF at page ${page}`}
          onClick={(e) => e.stopPropagation()}
        >
          p.{page}
          <ExternalLink className="h-2.5 w-2.5" />
        </Link>
      </div>

      {/* Chunk nodes indented under the page header */}
      {isOpen &&
        chunks.map((chunk) => (
          <ChunkTreeNode
            key={chunk.chunk_id}
            chunk={chunk}
            entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
            documentId={documentId}
            documentTitle={documentTitle}
            depth={2}
            isSelected={selectedChunkId === chunk.chunk_id}
            onSelect={onSelect}
          />
        ))}
    </div>
  );
}

// ============================================================================
// Chunk tree node
// ============================================================================

interface ChunkTreeNodeProps {
  chunk: FullLineageChunk;
  entities: EntityLineage[];
  /** Document ID — needed to build deeplink URL. */
  documentId: string;
  documentTitle: string;
  depth: number;
  /** Whether this chunk is currently selected (highlighted in content panel). */
  isSelected?: boolean;
  /** Callback fired when the chunk row is clicked. */
  onSelect?: (chunkId: string, startLine?: number, endLine?: number, page?: number) => void;
}

function ChunkTreeNode({
  chunk,
  entities,
  documentId,
  documentTitle,
  depth,
  isSelected,
  onSelect,
}: ChunkTreeNodeProps) {
  const { t } = useTranslation();
  const { ask } = useQueryHandoff();
  const lineInfo = chunk.start_line
    ? `L${chunk.start_line}–${chunk.end_line ?? '?'}`
    : `#${chunk.chunk_index}`;

  const entityCount = chunk.entity_ids?.length ?? entities.length;

  // SPEC-033: include page attribution when calling parent's onSelect.
  // This drives the PDF viewer to the correct page via URL update.
  const handleSelect = useCallback(() => {
    onSelect?.(chunk.chunk_id, chunk.start_line, chunk.end_line, chunk.page_start);
  }, [onSelect, chunk.chunk_id, chunk.start_line, chunk.end_line, chunk.page_start]);

  const handleAsk = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      const page =
        chunk.page_start !== undefined && chunk.page_start >= 1
          ? chunk.page_start
          : undefined;
      ask({
        kind: 'document',
        documentId,
        title: documentTitle,
        page,
        chunkId: chunk.chunk_id,
        startLine: chunk.start_line,
        endLine: chunk.end_line,
      });
    },
    [
      ask,
      chunk.chunk_id,
      chunk.end_line,
      chunk.page_start,
      chunk.start_line,
      documentId,
      documentTitle,
    ],
  );

  // Build the page deeplink when page_start is present (SPEC-033 FR-004 / SPEC-135).
  const pageUrl =
    chunk.page_start !== undefined && chunk.page_start > 0
      ? buildDocumentPageUrl(documentId, chunk.chunk_id, chunk.page_start)
      : null;
  const pageBadge = formatChunkPageBadge(chunk.page_start, chunk.page_end);
  const pageAria =
    chunk.page_start !== undefined &&
    chunk.page_end !== undefined &&
    chunk.page_end > chunk.page_start
      ? `Pages ${chunk.page_start} to ${chunk.page_end}`
      : `Page ${chunk.page_start}`;

  return (
    <TreeNode
      icon={<Layers className="h-3 w-3" />}
      label={`Chunk ${chunk.chunk_index}`}
      badge={pageUrl ? undefined : `${lineInfo} • ${entityCount} ent`}
      badgeRight={
        <span className="flex items-center gap-1 text-xs text-muted-foreground">
          {pageUrl ? (
            <>
              <span>{lineInfo} · {entityCount} ent</span>
              <Link
                href={pageUrl}
                className={cn(
                  'inline-flex items-center gap-0.5 text-xs font-medium',
                  'text-primary hover:underline',
                  'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50',
                  'rounded-sm px-1 py-0.5 leading-none',
                )}
                aria-label={pageAria}
                title={`Open PDF at page ${chunk.page_start}`}
                data-testid="chunk-page-badge"
                onClick={(e) => e.stopPropagation()}
              >
                {pageBadge}
              </Link>
            </>
          ) : null}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="h-5 w-5 shrink-0 opacity-60 hover:opacity-100"
            title={t('documents.detail.askAboutPage', 'Ask about this page')}
            aria-label={t('documents.detail.askAboutPage', 'Ask about this page')}
            data-testid="hierarchy-chunk-ask"
            onClick={handleAsk}
          >
            <MessageSquareText className="h-3 w-3" />
          </Button>
        </span>
      }
      depth={depth}
      isSelected={isSelected}
      chunkId={chunk.chunk_id}
      onSelect={handleSelect}
    >
      {entities.length === 0 ? (
        <p className="text-xs text-muted-foreground pl-6 py-0.5">
          No entities
        </p>
      ) : (
        entities.map((ent, idx) => (
          // WHY: Use both ent.id and idx to guarantee uniqueness when the backend
          // returns duplicate entity_ids in the same chunk's entity list.
          <EntityLeafNode key={`${ent.id ?? ent.name}_${idx}`} entity={ent} depth={depth + 1} />
        ))
      )}
    </TreeNode>
  );
}

// ============================================================================
// Entity leaf node
// ============================================================================

interface EntityLeafNodeProps {
  entity: EntityLineage;
  depth: number;
}

function EntityLeafNode({ entity, depth }: EntityLeafNodeProps) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 py-1 px-2 rounded text-xs hover:bg-muted/40 transition-colors'
      )}
      style={{ paddingLeft: `${(depth + 1) * 16}px` }}
    >
      <Tag className="h-3 w-3 shrink-0 text-muted-foreground" />
      <span className="font-medium truncate" title={entity.name}>
        {entity.name}
      </span>
      <Badge variant="outline" className="text-xs px-1.5 py-0 shrink-0">
        {entity.entity_type}
      </Badge>
      {entity.extraction_count > 1 && (
        <Badge variant="secondary" className="text-xs px-1.5 py-0 shrink-0">
          ×{entity.extraction_count}
        </Badge>
      )}
    </div>
  );
}

// ============================================================================
// Generic tree node (collapsible)
// ============================================================================

interface TreeNodeProps {
  icon: React.ReactNode;
  label: string;
  badge?: string;
  /** Right-side slot for custom badge content (overrides `badge` when both set). */
  badgeRight?: React.ReactNode;
  defaultOpen?: boolean;
  depth: number;
  children?: React.ReactNode;
  /** Highlight this node as selected (chunk highlight feature). */
  isSelected?: boolean;
  chunkId?: string;
  /** Extra action fired when the node row is clicked (in addition to toggle). */
  onSelect?: () => void;
}

function TreeNode({
  icon,
  label,
  badge,
  badgeRight,
  defaultOpen = false,
  depth,
  children,
  isSelected,
  chunkId,
  onSelect,
}: TreeNodeProps) {
  const [open, setOpen] = useState(defaultOpen);
  const toggle = useCallback(() => {
    setOpen((p) => !p);
    onSelect?.();
  }, [onSelect]);

  return (
    <div>
      <div
        className={cn(
          'flex items-center gap-1.5 w-full py-1.5 px-2 rounded text-sm',
          'hover:bg-muted/50 transition-colors',
          isSelected && 'bg-yellow-100 dark:bg-yellow-900/30 border-l-2 border-yellow-500 dark:border-yellow-400 font-semibold text-yellow-900 dark:text-yellow-100',
        )}
        style={{ paddingLeft: `${depth * 16}px` }}
      >
        <button
          type="button"
          onClick={toggle}
          data-testid={chunkId ? 'hierarchy-chunk-row' : undefined}
          data-chunk-id={chunkId}
          aria-current={isSelected ? 'true' : undefined}
          className={cn(
            'flex min-w-0 flex-1 items-center gap-1.5 text-left',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50',
          )}
        >
          {open ? (
            <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
          )}
          <span className="shrink-0">{icon}</span>
          <span className="font-medium truncate">{label}</span>
          {!badgeRight && badge ? (
            <span className="text-xs text-muted-foreground ml-auto shrink-0">
              {badge}
            </span>
          ) : null}
        </button>
        {badgeRight ? (
          <span className="ml-auto shrink-0">{badgeRight}</span>
        ) : null}
      </div>
      {open && <div>{children}</div>}
    </div>
  );
}
