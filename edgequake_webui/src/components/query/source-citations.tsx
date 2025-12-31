'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import {
    HoverCard,
    HoverCardContent,
    HoverCardTrigger,
} from '@/components/ui/hover-card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { QueryContext } from '@/types';
import { ChevronDown, ChevronUp, ExternalLink, FileText } from 'lucide-react';
import { useState } from 'react';

interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (documentId: string) => void;
}

export function SourceCitations({
  context,
  onEntityClick,
  onDocumentClick,
}: SourceCitationsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const hasChunks = context.chunks && context.chunks.length > 0;
  const hasEntities = context.entities && context.entities.length > 0;
  const hasRelationships = context.relationships && context.relationships.length > 0;

  if (!hasChunks && !hasEntities && !hasRelationships) {
    return null;
  }

  // Group chunks by document
  const chunksByDocument = context.chunks?.reduce((acc, chunk) => {
    if (!acc[chunk.document_id]) {
      acc[chunk.document_id] = [];
    }
    acc[chunk.document_id].push(chunk);
    return acc;
  }, {} as Record<string, typeof context.chunks>) || {};

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="w-full flex items-center justify-between text-muted-foreground hover:text-foreground"
        >
          <span className="flex items-center gap-2">
            <FileText className="h-3.5 w-3.5" />
            <span className="text-xs">
              Sources: {context.chunks?.length || 0} chunks · {context.entities?.length || 0} entities
            </span>
          </span>
          {isExpanded ? (
            <ChevronUp className="h-3.5 w-3.5" />
          ) : (
            <ChevronDown className="h-3.5 w-3.5" />
          )}
        </Button>
      </CollapsibleTrigger>

      <CollapsibleContent className="mt-2 space-y-3">
        {/* Document Chunks */}
        {hasChunks && (
          <div className="space-y-2">
            <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Source Documents
            </h4>
            <ScrollArea className="max-h-[200px]">
              <div className="space-y-2 pr-2">
                {Object.entries(chunksByDocument).map(([docId, chunks]) => (
                  <Card key={docId} className="bg-muted/30">
                    <CardContent className="p-2">
                      <div className="flex items-center justify-between mb-1">
                        <button
                          onClick={() => onDocumentClick?.(docId)}
                          className="text-xs font-medium truncate max-w-[200px] hover:text-primary flex items-center gap-1"
                        >
                          <FileText className="h-3 w-3" />
                          {docId.slice(0, 8)}...
                          <ExternalLink className="h-2.5 w-2.5" />
                        </button>
                        <Badge variant="secondary" className="text-[10px]">
                          {chunks.length} chunk{chunks.length !== 1 ? 's' : ''}
                        </Badge>
                      </div>
                      <div className="space-y-1">
                        {chunks.slice(0, 2).map((chunk, idx) => (
                          <HoverCard key={idx}>
                            <HoverCardTrigger asChild>
                              <p className="text-xs text-muted-foreground line-clamp-2 cursor-help hover:text-foreground">
                                {chunk.content.slice(0, 100)}...
                              </p>
                            </HoverCardTrigger>
                            <HoverCardContent className="w-80">
                              <div className="space-y-2">
                                <div className="flex items-center justify-between">
                                  <span className="text-sm font-medium">Source Chunk</span>
                                  <Badge variant="outline" className="text-[10px]">
                                    Score: {(chunk.score * 100).toFixed(0)}%
                                  </Badge>
                                </div>
                                <p className="text-xs text-muted-foreground whitespace-pre-wrap">
                                  {chunk.content}
                                </p>
                              </div>
                            </HoverCardContent>
                          </HoverCard>
                        ))}
                        {chunks.length > 2 && (
                          <p className="text-[10px] text-muted-foreground">
                            +{chunks.length - 2} more chunks
                          </p>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </div>
        )}

        {/* Related Entities */}
        {hasEntities && (
          <div className="space-y-2">
            <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Related Entities
            </h4>
            <div className="flex flex-wrap gap-1">
              {context.entities?.slice(0, 10).map((entity) => (
                <HoverCard key={entity.id}>
                  <HoverCardTrigger asChild>
                    <Badge
                      variant="secondary"
                      className="cursor-pointer hover:bg-secondary/80 text-xs"
                      onClick={() => onEntityClick?.(entity.id)}
                    >
                      {entity.label}
                    </Badge>
                  </HoverCardTrigger>
                  <HoverCardContent className="w-72">
                    <div className="space-y-2">
                      <p className="text-sm font-medium">{entity.label}</p>
                      <p className="text-xs text-muted-foreground">
                        Relevance: {(entity.relevance * 100).toFixed(0)}%
                      </p>
                      {/* Source Citation Link */}
                      {(entity.source_file_path || entity.source_document_id) && (
                        <div className="pt-1 border-t border-border/50">
                          <p className="text-[10px] text-muted-foreground uppercase tracking-wide mb-1">
                            Source
                          </p>
                          <button
                            onClick={() => entity.source_document_id && onDocumentClick?.(entity.source_document_id)}
                            className="text-xs text-primary hover:underline flex items-center gap-1 truncate max-w-full"
                          >
                            <FileText className="h-3 w-3 flex-shrink-0" />
                            <span className="truncate">
                              {entity.source_file_path 
                                ? entity.source_file_path.split('/').pop() 
                                : entity.source_document_id?.slice(0, 12) + '...'}
                            </span>
                            <ExternalLink className="h-2.5 w-2.5 flex-shrink-0" />
                          </button>
                        </div>
                      )}
                      <Button
                        variant="link"
                        size="sm"
                        className="p-0 h-auto text-xs"
                        onClick={() => onEntityClick?.(entity.id)}
                      >
                        View in graph →
                      </Button>
                    </div>
                  </HoverCardContent>
                </HoverCard>
              ))}
              {(context.entities?.length || 0) > 10 && (
                <Badge variant="outline" className="text-[10px]">
                  +{(context.entities?.length || 0) - 10} more
                </Badge>
              )}
            </div>
          </div>
        )}

        {/* Relationships */}
        {hasRelationships && (
          <div className="space-y-2">
            <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Key Relationships
            </h4>
            <div className="space-y-1">
              {context.relationships?.slice(0, 5).map((rel, idx) => (
                <HoverCard key={idx}>
                  <HoverCardTrigger asChild>
                    <div
                      className="flex items-center gap-1 text-xs text-muted-foreground cursor-help hover:bg-muted/50 rounded px-1 py-0.5"
                    >
                      <span
                        className="font-medium hover:text-primary cursor-pointer"
                        onClick={(e) => {
                          e.stopPropagation();
                          onEntityClick?.(rel.source);
                        }}
                      >
                        {rel.source.slice(0, 15)}
                      </span>
                      <span className="text-primary">→</span>
                      <Badge variant="outline" className="text-[10px]">
                        {rel.type}
                      </Badge>
                      <span className="text-primary">→</span>
                      <span
                        className="font-medium hover:text-primary cursor-pointer"
                        onClick={(e) => {
                          e.stopPropagation();
                          onEntityClick?.(rel.target);
                        }}
                      >
                        {rel.target.slice(0, 15)}
                      </span>
                    </div>
                  </HoverCardTrigger>
                  <HoverCardContent className="w-60">
                    <div className="space-y-2">
                      <p className="text-sm font-medium">
                        {rel.source} → {rel.target}
                      </p>
                      <Badge variant="secondary" className="text-[10px]">
                        {rel.type}
                      </Badge>
                      <p className="text-xs text-muted-foreground">
                        Relevance: {(rel.relevance * 100).toFixed(0)}%
                      </p>
                      {/* Source Citation Link */}
                      {(rel.source_file_path || rel.source_document_id) && (
                        <div className="pt-1 border-t border-border/50">
                          <p className="text-[10px] text-muted-foreground uppercase tracking-wide mb-1">
                            Source
                          </p>
                          <button
                            onClick={() => rel.source_document_id && onDocumentClick?.(rel.source_document_id)}
                            className="text-xs text-primary hover:underline flex items-center gap-1 truncate max-w-full"
                          >
                            <FileText className="h-3 w-3 flex-shrink-0" />
                            <span className="truncate">
                              {rel.source_file_path 
                                ? rel.source_file_path.split('/').pop() 
                                : rel.source_document_id?.slice(0, 12) + '...'}
                            </span>
                            <ExternalLink className="h-2.5 w-2.5 flex-shrink-0" />
                          </button>
                        </div>
                      )}
                    </div>
                  </HoverCardContent>
                </HoverCard>
              ))}
              {(context.relationships?.length || 0) > 5 && (
                <p className="text-[10px] text-muted-foreground">
                  +{(context.relationships?.length || 0) - 5} more relationships
                </p>
              )}
            </div>
          </div>
        )}
      </CollapsibleContent>
    </Collapsible>
  );
}

// Inline citation component for use within markdown
interface InlineCitationProps {
  index: number;
  chunk: {
    content: string;
    document_id: string;
    score: number;
  };
}

export function InlineCitation({ index, chunk }: InlineCitationProps) {
  return (
    <HoverCard>
      <HoverCardTrigger asChild>
        <sup className="cursor-help text-primary hover:text-primary/80 font-medium">
          [{index}]
        </sup>
      </HoverCardTrigger>
      <HoverCardContent className="w-80">
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium flex items-center gap-1">
              <FileText className="h-3 w-3" />
              Source #{index}
            </span>
            <Badge variant="outline" className="text-[10px]">
              {(chunk.score * 100).toFixed(0)}% match
            </Badge>
          </div>
          <p className="text-xs text-muted-foreground line-clamp-4">
            {chunk.content}
          </p>
          <p className="text-[10px] text-muted-foreground truncate">
            Document: {chunk.document_id}
          </p>
        </div>
      </HoverCardContent>
    </HoverCard>
  );
}

export default SourceCitations;
