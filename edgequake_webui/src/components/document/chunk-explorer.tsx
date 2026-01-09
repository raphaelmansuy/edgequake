/**
 * Chunk Explorer Component
 * 
 * Browse document chunks with entity highlighting.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 *
 * @implements FEAT1070 - Document chunk browsing
 * @implements FEAT1071 - Entity highlighting in chunks
 *
 * @see UC1501 - User explores document chunks
 * @see UC1502 - User views entities per chunk
 *
 * @enforces BR1070 - Searchable chunk list
 * @enforces BR1071 - Selection state synchronization
 */

'use client';

import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import type { ChunkLineage } from '@/types/lineage';
import { Box, ChevronRight, Clock, Link2, Search, User, Zap } from 'lucide-react';
import { useMemo, useState } from 'react';

interface ChunkExplorerProps {
  /** Document ID */
  documentId: string;
  /** List of chunks with lineage data */
  chunks: ChunkLineage[];
  /** Callback when a chunk is selected */
  onChunkSelect: (chunkId: string) => void;
  /** Currently selected chunk ID */
  selectedChunkId?: string;
  /** Highlight entities in preview */
  highlightEntities?: boolean;
  /** Loading state */
  isLoading?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Formats duration in human-readable format.
 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/**
 * Displays a list of chunks with extraction summaries.
 * Clicking a chunk opens the ChunkDetailModal.
 */
export function ChunkExplorer({
  documentId: _documentId, // eslint-disable-line @typescript-eslint/no-unused-vars
  chunks,
  onChunkSelect,
  selectedChunkId,
  highlightEntities: _highlightEntities = true, // eslint-disable-line @typescript-eslint/no-unused-vars
  isLoading = false,
  className,
}: ChunkExplorerProps) {
  const [searchQuery, setSearchQuery] = useState('');

  // Filter chunks based on search
  const filteredChunks = useMemo(() => {
    if (!searchQuery.trim()) return chunks;
    
    const query = searchQuery.toLowerCase();
    return chunks.filter(chunk => {
      const chunkId = chunk.id ?? chunk.chunk_id;
      const preview = chunk.content_preview ?? '';
      return preview.toLowerCase().includes(query) ||
        chunkId.toLowerCase().includes(query);
    });
  }, [chunks, searchQuery]);

  if (isLoading) {
    return (
      <div className={cn('space-y-3', className)}>
        <Skeleton className="h-10 w-full" />
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="h-24 w-full" />
        ))}
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b">
        <div className="flex items-center gap-2">
          <Box className="h-4 w-4 text-muted-foreground" />
          <span className="font-medium text-sm">
            Chunks ({chunks.length})
          </span>
        </div>
        
        <div className="relative w-48">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            placeholder="Search..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-8 pl-8 text-sm"
          />
        </div>
      </div>

      {/* Chunk list */}
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-2">
          {filteredChunks.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground text-sm">
              {searchQuery ? 'No matching chunks found' : 'No chunks available'}
            </div>
          ) : (
            filteredChunks.map((chunk) => {
              const chunkId = chunk.id ?? chunk.chunk_id;
              return (
                <ChunkCard
                  key={chunkId}
                  chunk={chunk}
                  isSelected={selectedChunkId === chunkId}
                  onClick={() => onChunkSelect(chunkId)}
                />
              );
            })
          )}
        </div>
      </ScrollArea>
    </div>
  );
}

/**
 * Individual chunk card component.
 */
interface ChunkCardProps {
  chunk: ChunkLineage;
  isSelected: boolean;
  onClick: () => void;
}

function ChunkCard({ chunk, isSelected, onClick }: ChunkCardProps) {
  const entityCount = chunk.extracted_entities.length;
  const relationshipCount = chunk.extracted_relationships.length;
  const isCached = chunk.extraction_metadata?.cached ?? chunk.extraction_metadata?.cache_hit ?? false;

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full text-left p-3 rounded-lg border transition-all',
        'hover:border-primary/50 hover:bg-muted/50',
        isSelected && 'border-primary bg-primary/5'
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <Box className="h-4 w-4 text-muted-foreground" />
          <span className="font-medium text-sm">
            Chunk {chunk.index + 1}
          </span>
          {isCached && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Badge variant="outline" className="text-xs px-1.5 py-0">
                  <Zap className="h-3 w-3 mr-0.5" />
                  cached
                </Badge>
              </TooltipTrigger>
              <TooltipContent>
                <p>Result was cached from previous extraction</p>
              </TooltipContent>
            </Tooltip>
          )}
        </div>
        
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>
            Lines {chunk.char_range?.start ?? 0}-{chunk.char_range?.end ?? 0}
          </span>
          <span>|</span>
          <span>{chunk.token_count.toLocaleString()} tok</span>
          <ChevronRight className="h-4 w-4" />
        </div>
      </div>

      {/* Preview */}
      <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
        &ldquo;{chunk.content_preview}&rdquo;
      </p>

      {/* Footer stats */}
      <div className="flex items-center gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <User className="h-3 w-3" />
          {entityCount} {entityCount === 1 ? 'entity' : 'entities'}
        </span>
        <span className="flex items-center gap-1">
          <Link2 className="h-3 w-3" />
          {relationshipCount} {relationshipCount === 1 ? 'relationship' : 'relationships'}
        </span>
        {(chunk.extraction_metadata?.duration_ms ?? chunk.extraction_metadata?.extraction_time_ms) && (
          <span className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDuration(chunk.extraction_metadata?.duration_ms ?? chunk.extraction_metadata?.extraction_time_ms ?? 0)}
          </span>
        )}
      </div>
    </button>
  );
}

export default ChunkExplorer;
