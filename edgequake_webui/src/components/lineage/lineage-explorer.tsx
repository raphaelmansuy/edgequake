/**
 * @module LineageExplorer
 * @description Main container for lineage visualization with multiple view modes.
 * Based on WebUI Specification Document WEBUI-006 (15-webui-lineage-viz.md)
 * 
 * @implements UC0301 - User explores document chunks and origins
 * @implements UC0304 - User views lineage tree visualization
 * @implements FEAT0701 - Document lineage visualization
 * @implements FEAT0703 - Multiple view modes (tree, table, graph)
 * @implements FEAT0704 - Chunk search and filtering
 * 
 * @enforces BR0701 - Lineage preserved for all entities
 * @enforces BR0703 - View mode persists during session
 * 
 * @see {@link specs/WEBUI-006.md} for specification
 */

'use client';

import { ChunkDetailModal } from '@/components/document/chunk-detail-modal';
import { ChunkExplorer } from '@/components/document/chunk-explorer';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useChunkDetail, useDocumentLineage } from '@/hooks/use-lineage';
import { cn } from '@/lib/utils';
import type { EntityLineage } from '@/types/lineage';
import {
    AlertCircle,
    Box,
    Download,
    Network,
    Search,
    Table,
    TreeDeciduous,
    User
} from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';

interface LineageExplorerProps {
  /** Document ID to show lineage for */
  documentId: string;
  /** Initial view mode */
  initialView?: 'tree' | 'graph' | 'table';
  /** Callback when chunk is clicked */
  onChunkClick?: (chunkId: string) => void;
  /** Callback when entity is clicked */
  onEntityClick?: (entityId: string) => void;
  /** Custom class name */
  className?: string;
}

/**
 * Main lineage exploration component with tree, graph, and table views.
 */
export function LineageExplorer({
  documentId,
  initialView = 'tree',
  onChunkClick,
  onEntityClick,
  className,
}: LineageExplorerProps) {
  const [view, setView] = useState(initialView);
  const [filter, setFilter] = useState('');
  const [selectedChunkId, setSelectedChunkId] = useState<string | null>(null);
  const [chunkModalOpen, setChunkModalOpen] = useState(false);

  // Fetch lineage data
  const { data: lineage, isLoading, error } = useDocumentLineage(documentId);
  
  // Fetch selected chunk detail
  const { data: selectedChunk, isLoading: isChunkLoading } = useChunkDetail(
    chunkModalOpen ? selectedChunkId : null
  );

  // Handle chunk selection
  const handleChunkSelect = useCallback((chunkId: string) => {
    setSelectedChunkId(chunkId);
    setChunkModalOpen(true);
    onChunkClick?.(chunkId);
  }, [onChunkClick]);

  // Handle entity click
  const handleEntityClick = useCallback((entityId: string) => {
    onEntityClick?.(entityId);
  }, [onEntityClick]);

  // Close chunk modal
  const handleCloseChunkModal = useCallback(() => {
    setChunkModalOpen(false);
  }, []);

  // Filter lineage data
  const filteredLineage = useMemo(() => {
    if (!lineage || !filter.trim()) return lineage;
    
    const query = filter.toLowerCase();
    
    // Filter entities that match
    const matchingEntityIds = new Set(
      lineage.entities
        .filter(e => 
          e.name.toLowerCase().includes(query) ||
          e.entity_type.toLowerCase().includes(query)
        )
        .map(e => e.id)
    );
    
    // Filter chunks that have matching entities
    const filteredChunks = lineage.chunks.filter(chunk =>
      chunk.extracted_entities.some(id => matchingEntityIds.has(id)) ||
      (chunk.content_preview ?? '').toLowerCase().includes(query)
    );

    return {
      ...lineage,
      chunks: filteredChunks,
      entities: lineage.entities.filter(e => matchingEntityIds.has(e.id)),
    };
  }, [lineage, filter]);

  // Export lineage data
  const handleExport = useCallback((format: 'json' | 'csv') => {
    if (!lineage) return;

    let content: string;
    let filename: string;
    let mimeType: string;

    if (format === 'json') {
      content = JSON.stringify(lineage, null, 2);
      filename = `lineage-${documentId}.json`;
      mimeType = 'application/json';
    } else {
      // CSV export
      const headers = ['Chunk ID', 'Chunk Index', 'Entity Name', 'Entity Type', 'Description'];
      const rows: string[][] = [];
      
      lineage.chunks.forEach(chunk => {
        const chunkId = chunk.id ?? chunk.chunk_id;
        lineage.entities
          .filter(e => chunk.extracted_entities.includes(e.id))
          .forEach(entity => {
            rows.push([
              chunkId,
              String(chunk.index + 1),
              entity.name,
              entity.entity_type,
              entity.description || '',
            ]);
          });
      });

      content = [
        headers.join(','),
        ...rows.map(row => row.map(cell => `"${cell.replace(/"/g, '""')}"`).join(',')),
      ].join('\n');
      filename = `lineage-${documentId}.csv`;
      mimeType = 'text/csv';
    }

    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }, [lineage, documentId]);

  // Loading state
  if (isLoading) {
    return (
      <div className={cn('flex flex-col h-full', className)}>
        <div className="flex items-center justify-between p-4 border-b">
          <Skeleton className="h-10 w-48" />
          <Skeleton className="h-10 w-64" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-full w-full" />
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <Alert variant="destructive" className={className}>
        <AlertCircle className="h-4 w-4" />
        <AlertDescription>
          Failed to load lineage data: {(error as Error).message}
        </AlertDescription>
      </Alert>
    );
  }

  // No data state
  if (!lineage) {
    return (
      <div className={cn('flex items-center justify-center h-full', className)}>
        <p className="text-muted-foreground">No lineage data available</p>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Toolbar */}
      <div className="flex items-center justify-between p-4 border-b gap-4">
        {/* View tabs */}
        <Tabs value={view} onValueChange={(v) => setView(v as typeof view)}>
          <TabsList>
            <TabsTrigger value="tree">
              <TreeDeciduous className="h-4 w-4 mr-1.5" />
              Tree
            </TabsTrigger>
            <TabsTrigger value="graph" disabled>
              <Network className="h-4 w-4 mr-1.5" />
              Graph
            </TabsTrigger>
            <TabsTrigger value="table">
              <Table className="h-4 w-4 mr-1.5" />
              Table
            </TabsTrigger>
          </TabsList>
        </Tabs>

        {/* Search and export */}
        <div className="flex items-center gap-2">
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Filter entities..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="pl-9"
            />
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon">
                <Download className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => handleExport('json')}>
                Export as JSON
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleExport('csv')}>
                Export as CSV
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* View content */}
      <div className="flex-1 overflow-hidden">
        {view === 'tree' && (
          <LineageTreeView
            lineage={filteredLineage}
            onChunkSelect={handleChunkSelect}
            onEntityClick={handleEntityClick}
            selectedChunkId={selectedChunkId}
          />
        )}
        {view === 'table' && (
          <LineageTableView
            lineage={filteredLineage}
            onChunkClick={handleChunkSelect}
            onEntityClick={handleEntityClick}
          />
        )}
      </div>

      {/* Statistics footer */}
      <LineageStatisticsBar 
        chunkCount={lineage.chunks.length}
        entityCount={lineage.entities.length}
        relationshipCount={lineage.relationships?.length ?? 0}
      />

      {/* Chunk detail modal */}
      <ChunkDetailModal
        chunk={selectedChunk ?? null}
        isOpen={chunkModalOpen}
        onClose={handleCloseChunkModal}
        onEntityClick={handleEntityClick}
        isLoading={isChunkLoading}
      />
    </div>
  );
}

/**
 * Tree view implementation.
 */
function LineageTreeView({
  lineage,
  onChunkSelect,
  onEntityClick,
  selectedChunkId,
}: {
  lineage: ReturnType<typeof useDocumentLineage>['data'];
  onChunkSelect: (chunkId: string) => void;
  onEntityClick: (entityId: string) => void;
  selectedChunkId: string | null;
}) {
  if (!lineage) return null;

  return (
    <div className="flex h-full">
      {/* Chunk explorer */}
      <div className="w-1/2 border-r">
        <ChunkExplorer
          documentId={lineage.document_id}
          chunks={lineage.chunks}
          onChunkSelect={onChunkSelect}
          selectedChunkId={selectedChunkId ?? undefined}
          className="h-full"
        />
      </div>

      {/* Entity list */}
      <div className="w-1/2">
        <EntityList
          entities={lineage.entities}
          onEntityClick={onEntityClick}
        />
      </div>
    </div>
  );
}

/**
 * Entity list component.
 */
function EntityList({
  entities,
  onEntityClick,
}: {
  entities: EntityLineage[];
  onEntityClick: (entityId: string) => void;
}) {
  // Group entities by type
  const grouped = useMemo(() => {
    const groups: Record<string, EntityLineage[]> = {};
    entities.forEach(entity => {
      const type = entity.entity_type;
      if (!groups[type]) groups[type] = [];
      groups[type].push(entity);
    });
    return groups;
  }, [entities]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 p-3 border-b">
        <User className="h-4 w-4 text-muted-foreground" />
        <span className="font-medium text-sm">
          Entities ({entities.length})
        </span>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-3 space-y-4">
          {Object.entries(grouped).map(([type, typeEntities]) => (
            <div key={type}>
              <div className="flex items-center gap-2 mb-2">
                <Badge variant="outline">{type}</Badge>
                <span className="text-xs text-muted-foreground">
                  ({typeEntities.length})
                </span>
              </div>
              <div className="space-y-1 pl-2">
                {typeEntities.map(entity => (
                  <button
                    key={entity.id}
                    type="button"
                    onClick={() => onEntityClick(entity.id)}
                    className={cn(
                      'w-full text-left px-2 py-1.5 rounded text-sm',
                      'hover:bg-muted transition-colors'
                    )}
                  >
                    <span className="font-medium">{entity.name}</span>
                    {entity.merged_from && (
                      <Badge variant="secondary" className="ml-2 text-xs">
                        merged
                      </Badge>
                    )}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}

/**
 * Table view implementation.
 */
function LineageTableView({
  lineage,
  onChunkClick,
  onEntityClick,
}: {
  lineage: ReturnType<typeof useDocumentLineage>['data'];
  onChunkClick: (chunkId: string) => void;
  onEntityClick: (entityId: string) => void;
}) {
  // Flatten data for table - must be before any early returns
  const rows = useMemo(() => {
    if (!lineage) return [];
    
    const result: Array<{
      chunkId: string;
      chunkIndex: number;
      entityId: string;
      entityName: string;
      entityType: string;
    }> = [];

    lineage.chunks.forEach(chunk => {
      const chunkId = chunk.id ?? chunk.chunk_id;
      lineage.entities
        .filter(e => chunk.extracted_entities.includes(e.id))
        .forEach(entity => {
          result.push({
            chunkId,
            chunkIndex: chunk.index + 1,
            entityId: entity.id,
            entityName: entity.name,
            entityType: entity.entity_type,
          });
        });
    });

    return result;
  }, [lineage]);

  if (!lineage) return null;

  return (
    <ScrollArea className="h-full">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-background border-b">
          <tr>
            <th className="px-4 py-3 text-left font-medium">Chunk</th>
            <th className="px-4 py-3 text-left font-medium">Entity</th>
            <th className="px-4 py-3 text-left font-medium">Type</th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {rows.map((row, index) => (
            <tr key={`${row.chunkId}-${row.entityId}-${index}`} className="hover:bg-muted/50">
              <td className="px-4 py-3">
                <button
                  type="button"
                  onClick={() => onChunkClick(row.chunkId)}
                  className="text-primary hover:underline"
                >
                  Chunk {row.chunkIndex}
                </button>
              </td>
              <td className="px-4 py-3">
                <button
                  type="button"
                  onClick={() => onEntityClick(row.entityId)}
                  className="text-primary hover:underline"
                >
                  {row.entityName}
                </button>
              </td>
              <td className="px-4 py-3">
                <Badge variant="outline">{row.entityType}</Badge>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </ScrollArea>
  );
}

/**
 * Statistics bar at bottom of lineage explorer.
 */
function LineageStatisticsBar({
  chunkCount,
  entityCount,
  relationshipCount,
}: {
  chunkCount: number;
  entityCount: number;
  relationshipCount: number;
}) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-t bg-muted/30 text-xs text-muted-foreground">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1">
          <Box className="h-3 w-3" />
          {chunkCount} chunks
        </span>
        <span className="flex items-center gap-1">
          <User className="h-3 w-3" />
          {entityCount} entities
        </span>
        <span className="flex items-center gap-1">
          <Network className="h-3 w-3" />
          {relationshipCount} relationships
        </span>
      </div>
    </div>
  );
}

export default LineageExplorer;
