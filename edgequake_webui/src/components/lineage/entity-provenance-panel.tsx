/**
 * @module EntityProvenancePanel
 * @description Displays source information for an entity across documents.
 * Based on WebUI Specification Document WEBUI-006 (15-webui-lineage-viz.md)
 * 
 * @implements UC0302 - User views entity extraction provenance
 * @implements UC0305 - User navigates from entity to source chunk
 * @implements FEAT0702 - Entity-to-document tracing
 * @implements FEAT0705 - Related entity exploration
 * 
 * @enforces BR0702 - Chunk positions accurate to source
 * @enforces BR0704 - All extraction sources shown
 * 
 * @see {@link specs/WEBUI-006.md} for specification
 */

'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { useEntityProvenance } from '@/hooks/use-lineage';
import { cn } from '@/lib/utils';
import {
    Box,
    ChevronRight,
    FileText,
    Link2,
    User,
    X,
} from 'lucide-react';

interface EntityProvenancePanelProps {
  /** Entity ID to show provenance for */
  entityId: string;
  /** Callback when panel is closed */
  onClose: () => void;
  /** Callback when a document is clicked */
  onDocumentClick?: (documentId: string) => void;
  /** Callback when a chunk is clicked */
  onChunkClick?: (chunkId: string) => void;
  /** Callback when a related entity is clicked */
  onRelatedEntityClick?: (entityId: string) => void;
  /** Custom class name */
  className?: string;
}

/**
 * Displays provenance information for an entity including:
 * - Source documents and chunks
 * - Related entities
 * - Description history
 */
export function EntityProvenancePanel({
  entityId,
  onClose,
  onDocumentClick,
  onChunkClick,
  onRelatedEntityClick,
  className,
}: EntityProvenancePanelProps) {
  const { data: provenance, isLoading, error } = useEntityProvenance(entityId);

  if (isLoading) {
    return (
      <Card className={cn('h-full', className)}>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <Skeleton className="h-6 w-48" />
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-24 w-full" />
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={cn('h-full', className)}>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">Entity Provenance</CardTitle>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            Failed to load provenance: {(error as Error).message}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!provenance) {
    return (
      <Card className={cn('h-full', className)}>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">Entity Provenance</CardTitle>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent>
          <div className="text-muted-foreground text-sm">
            No provenance data found for entity.
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="flex flex-row items-center justify-between pb-2 shrink-0">
        <div className="flex items-center gap-2">
          <User className="h-5 w-5 text-muted-foreground" />
          <div>
            <CardTitle className="text-base">{provenance.entity_name}</CardTitle>
            <CardDescription className="flex items-center gap-2">
              <Badge variant="secondary" className="text-xs">
                {provenance.entity_type}
              </Badge>
              <span>•</span>
              <span>{provenance.total_extraction_count} extractions</span>
            </CardDescription>
          </div>
        </div>
        <Button variant="ghost" size="icon" onClick={onClose}>
          <X className="h-4 w-4" />
        </Button>
      </CardHeader>

      <ScrollArea className="flex-1">
        <CardContent className="space-y-6">
          {/* Description */}
          {provenance.description && (
            <div>
              <h4 className="text-sm font-medium mb-2 text-muted-foreground">Description</h4>
              <p className="text-sm">{provenance.description}</p>
            </div>
          )}

          <Separator />

          {/* Source Documents */}
          <div>
            <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
              <FileText className="h-4 w-4 text-muted-foreground" />
              Source Documents ({provenance.sources.length})
            </h4>
            <div className="space-y-2">
              {provenance.sources.map((source) => (
                <button
                  key={source.document_id}
                  type="button"
                  onClick={() => onDocumentClick?.(source.document_id)}
                  className={cn(
                    'w-full p-3 rounded-lg border text-left transition-all',
                    'hover:border-primary/50 hover:bg-muted/50'
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <FileText className="h-4 w-4 text-muted-foreground" />
                      <span className="font-medium text-sm truncate max-w-50">
                        {source.document_name || source.document_id.slice(0, 12) + '...'}
                      </span>
                    </div>
                    <ChevronRight className="h-4 w-4 text-muted-foreground" />
                  </div>
                  
                  {source.chunks.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                      {source.chunks.slice(0, 3).map((chunk) => (
                        <Badge
                          key={chunk.chunk_id}
                          variant="outline"
                          className="text-xs cursor-pointer"
                          onClick={(e) => {
                            e.stopPropagation();
                            onChunkClick?.(chunk.chunk_id);
                          }}
                        >
                          <Box className="h-3 w-3 mr-1" />
                          Chunk
                        </Badge>
                      ))}
                      {source.chunks.length > 3 && (
                        <Badge variant="outline" className="text-xs">
                          +{source.chunks.length - 3} more
                        </Badge>
                      )}
                    </div>
                  )}
                </button>
              ))}
              
              {provenance.sources.length === 0 && (
                <div className="text-center py-4 text-muted-foreground text-sm">
                  No source documents found
                </div>
              )}
            </div>
          </div>

          <Separator />

          {/* Related Entities */}
          <div>
            <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
              <Link2 className="h-4 w-4 text-muted-foreground" />
              Related Entities ({provenance.related_entities.length})
            </h4>
            <div className="space-y-2">
              {provenance.related_entities.slice(0, 10).map((related) => (
                <button
                  key={related.entity_id}
                  type="button"
                  onClick={() => onRelatedEntityClick?.(related.entity_id)}
                  className={cn(
                    'w-full p-2 rounded-lg border text-left transition-all',
                    'hover:border-primary/50 hover:bg-muted/50',
                    'flex items-center justify-between'
                  )}
                >
                  <div className="flex items-center gap-2">
                    <User className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium text-sm">
                      {related.entity_name}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="secondary" className="text-xs">
                      {related.relationship_type}
                    </Badge>
                    <ChevronRight className="h-4 w-4 text-muted-foreground" />
                  </div>
                </button>
              ))}
              
              {provenance.related_entities.length > 10 && (
                <div className="text-center text-sm text-muted-foreground">
                  +{provenance.related_entities.length - 10} more related entities
                </div>
              )}
              
              {provenance.related_entities.length === 0 && (
                <div className="text-center py-4 text-muted-foreground text-sm">
                  No related entities found
                </div>
              )}
            </div>
          </div>
        </CardContent>
      </ScrollArea>
    </Card>
  );
}

export default EntityProvenancePanel;
