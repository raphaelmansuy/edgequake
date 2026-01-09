/**
 * Chunk Detail Modal Component
 * 
 * Full chunk view with entities and relationships.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 *
 * @implements FEAT1080 - Chunk detail modal view
 * @implements FEAT1081 - Entity/relationship tabs
 *
 * @see UC1511 - User views chunk details
 * @see UC1512 - User copies chunk content
 *
 * @enforces BR1080 - Tab-based content organization
 * @enforces BR1081 - Accessible dialog structure
 */

'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import type { ChunkDetail, EntityLineage } from '@/types/lineage';
import {
    Box,
    Check,
    Clock,
    Copy,
    Cpu, DollarSign,
    FileText,
    Hash,
    User,
    Zap
} from 'lucide-react';
import { useState } from 'react';

interface ChunkDetailModalProps {
  /** Chunk detail data */
  chunk: ChunkDetail | null;
  /** Whether the modal is open */
  isOpen: boolean;
  /** Callback when modal closes */
  onClose: () => void;
  /** Callback when entity is clicked */
  onEntityClick?: (entityId: string) => void;
  /** Loading state */
  isLoading?: boolean;
}

/**
 * Formats token count with K/M suffix.
 */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
  return tokens.toString();
}

/**
 * Formats duration in human-readable format.
 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/**
 * Modal displaying full chunk details including content,
 * extracted entities, and relationships.
 */
export function ChunkDetailModal({
  chunk,
  isOpen,
  onClose,
  onEntityClick,
  isLoading = false,
}: ChunkDetailModalProps) {
  const [copied, setCopied] = useState(false);

  const handleCopyContent = async () => {
    if (!chunk?.content) return;
    
    await navigator.clipboard.writeText(chunk.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent className="max-w-3xl max-h-[85vh] flex flex-col">
        {isLoading || !chunk ? (
          <ChunkDetailSkeleton />
        ) : (
          <>
            <DialogHeader>
              <div className="flex items-center gap-2">
                <Box className="h-5 w-5 text-muted-foreground" />
                <DialogTitle>Chunk {chunk.index + 1}</DialogTitle>
                {(chunk.extraction_metadata?.cached ?? chunk.extraction_metadata?.cache_hit) && (
                  <Badge variant="outline" className="ml-2">
                    <Zap className="h-3 w-3 mr-1" />
                    Cached
                  </Badge>
                )}
              </div>
              <DialogDescription>
                Lines {chunk.char_range?.start ?? 0}-{chunk.char_range?.end ?? 0} • {chunk.token_count.toLocaleString()} tokens
              </DialogDescription>
            </DialogHeader>

            <Tabs defaultValue="content" className="flex-1 flex flex-col min-h-0">
              <TabsList className="w-full grid grid-cols-3">
                <TabsTrigger value="content">
                  <FileText className="h-4 w-4 mr-1.5" />
                  Content
                </TabsTrigger>
                <TabsTrigger value="entities">
                  <User className="h-4 w-4 mr-1.5" />
                  Entities ({chunk.entities?.length ?? 0})
                </TabsTrigger>
                <TabsTrigger value="metadata">
                  <Cpu className="h-4 w-4 mr-1.5" />
                  Metadata
                </TabsTrigger>
              </TabsList>

              {/* Content Tab */}
              <TabsContent value="content" className="flex-1 mt-4">
                <div className="relative">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute right-2 top-2"
                    onClick={handleCopyContent}
                  >
                    {copied ? (
                      <Check className="h-4 w-4" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                  <ScrollArea className="h-[400px] rounded-lg border bg-muted/30 p-4">
                    <pre className="text-sm whitespace-pre-wrap font-mono">
                      {chunk.content}
                    </pre>
                  </ScrollArea>
                </div>
              </TabsContent>

              {/* Entities Tab */}
              <TabsContent value="entities" className="flex-1 mt-4">
                <ScrollArea className="h-[400px]">
                  {chunk.entities && chunk.entities.length > 0 ? (
                    <div className="space-y-2 pr-4">
                      {chunk.entities.map((entity) => (
                        <EntityCard
                          key={entity.id}
                          entity={entity}
                          onClick={() => onEntityClick?.(entity.id)}
                        />
                      ))}
                    </div>
                  ) : (
                    <div className="flex items-center justify-center h-full text-muted-foreground">
                      No entities extracted from this chunk
                    </div>
                  )}
                </ScrollArea>
              </TabsContent>

              {/* Metadata Tab */}
              <TabsContent value="metadata" className="flex-1 mt-4">
                <ScrollArea className="h-[400px]">
                  <MetadataSection chunk={chunk} />
                </ScrollArea>
              </TabsContent>
            </Tabs>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * Entity card within chunk detail.
 */
function EntityCard({
  entity,
  onClick,
}: {
  entity: EntityLineage;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full text-left p-3 rounded-lg border bg-card',
        'hover:border-primary/50 hover:bg-muted/50 transition-all'
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <User className="h-4 w-4 text-muted-foreground shrink-0" />
            <span className="font-medium truncate">{entity.name}</span>
            <Badge variant="secondary" className="shrink-0">
              {entity.entity_type}
            </Badge>
          </div>
          {entity.description && (
            <p className="text-sm text-muted-foreground line-clamp-2">
              {entity.description}
            </p>
          )}
        </div>
        
        {entity.merged_from && entity.merged_from.length > 0 && (
          <Badge variant="outline" className="shrink-0">
            Merged ({entity.merged_from.length})
          </Badge>
        )}
      </div>
    </button>
  );
}

/**
 * Metadata section with extraction details.
 */
function MetadataSection({ chunk }: { chunk: ChunkDetail }) {
  const meta = chunk.extraction_metadata;

  const items = [
    { 
      label: 'Chunk ID', 
      value: chunk.id,
      icon: Hash 
    },
    { 
      label: 'Token Count', 
      value: formatTokens(chunk.token_count),
      icon: FileText 
    },
    { 
      label: 'Character Range', 
      value: `${chunk.char_range?.start ?? 0} - ${chunk.char_range?.end ?? 0}`,
      icon: Hash 
    },
  ];

  if (meta) {
    items.push(
      { label: 'Model', value: meta.model, icon: Cpu },
      { label: 'Duration', value: formatDuration(meta.duration_ms ?? meta.extraction_time_ms), icon: Clock },
      { label: 'Prompt Tokens', value: formatTokens(meta.prompt_tokens ?? meta.input_tokens), icon: FileText },
      { label: 'Completion Tokens', value: formatTokens(meta.completion_tokens ?? meta.output_tokens), icon: FileText },
    );
  }

  return (
    <div className="space-y-4 pr-4">
      <div className="grid grid-cols-2 gap-4">
        {items.map((item) => (
          <div key={item.label} className="space-y-1">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <item.icon className="h-3 w-3" />
              {item.label}
            </div>
            <div className="font-medium text-sm truncate" title={item.value}>
              {item.value}
            </div>
          </div>
        ))}
      </div>

      {meta && (
        <>
          <Separator />
          
          <div className="space-y-2">
            <h4 className="font-medium text-sm">Cost</h4>
            <div className="flex items-center gap-2 p-3 rounded-lg bg-muted/50">
              <DollarSign className="h-4 w-4 text-muted-foreground" />
              <span className="font-mono text-lg">
                ${(meta.cost_usd ?? 0).toFixed(4)}
              </span>
              {(meta.cached ?? meta.cache_hit) && (
                <Badge variant="secondary" className="ml-auto">
                  <Zap className="h-3 w-3 mr-1" />
                  Cached - No cost
                </Badge>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

/**
 * Loading skeleton for chunk detail.
 */
function ChunkDetailSkeleton() {
  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-4 w-48" />
      </div>
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-[400px] w-full" />
    </div>
  );
}

export default ChunkDetailModal;
