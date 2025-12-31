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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { QueryContext } from '@/types';
import {
    BookOpen,
    Brain,
    ChevronDown,
    ChevronUp,
    ExternalLink,
    FileText,
    Network,
    Sparkles
} from 'lucide-react';
import { useMemo, useState } from 'react';

interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (documentId: string) => void;
  onExploreGraph?: () => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

const calculateConfidence = (context: QueryContext): number => {
  const scores = [
    ...(context.chunks?.map(c => c.score) || []),
    ...(context.entities?.map(e => e.relevance) || []),
    ...(context.relationships?.map(r => r.relevance) || []),
  ];
  if (scores.length === 0) return 0;
  return scores.reduce((a, b) => a + b, 0) / scores.length;
};

const getConfidenceLabel = (score: number): { label: string; color: string; bgColor: string } => {
  if (score >= 0.8) return { label: 'High', color: 'text-emerald-600 dark:text-emerald-400', bgColor: 'bg-emerald-500' };
  if (score >= 0.6) return { label: 'Good', color: 'text-green-600 dark:text-green-400', bgColor: 'bg-green-500' };
  if (score >= 0.4) return { label: 'Medium', color: 'text-amber-600 dark:text-amber-400', bgColor: 'bg-amber-500' };
  return { label: 'Low', color: 'text-red-600 dark:text-red-400', bgColor: 'bg-red-500' };
};

// ─────────────────────────────────────────────────────────────────────────────
// Sub-Components
// ─────────────────────────────────────────────────────────────────────────────

const ConfidenceDots = ({ score, className = '' }: { score: number; className?: string }) => {
  const filled = Math.round(score * 5);
  const { bgColor } = getConfidenceLabel(score);
  return (
    <span 
      className={`inline-flex gap-0.5 items-center ${className}`} 
      title={`${Math.round(score * 100)}% confidence`}
      aria-label={`Confidence: ${Math.round(score * 100)}%`}
    >
      {[...Array(5)].map((_, i) => (
        <span
          key={i}
          className={`w-1.5 h-1.5 rounded-full transition-colors ${
            i < filled ? bgColor : 'bg-muted-foreground/20'
          }`}
        />
      ))}
    </span>
  );
};

// Documents Tab Component
const DocumentsTab = ({ 
  chunksByDocument, 
  onDocumentClick 
}: { 
  chunksByDocument: Record<string, NonNullable<QueryContext['chunks']>>;
  onDocumentClick?: (docId: string) => void;
}) => {
  const [showAll, setShowAll] = useState(false);
  const entries = Object.entries(chunksByDocument);
  const visibleEntries = showAll ? entries : entries.slice(0, 3);
  
  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
        <FileText className="h-8 w-8 mb-2 opacity-50" />
        <p className="text-sm">No source documents</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <ScrollArea className="max-h-[300px]">
        <div className="space-y-2 pr-2">
          {visibleEntries.map(([docId, chunks], index) => {
            const avgScore = chunks.reduce((acc, c) => acc + c.score, 0) / chunks.length;
            const { color: scoreColor } = getConfidenceLabel(avgScore);
            
            return (
              <Card 
                key={docId} 
                className="group bg-muted/30 hover:bg-muted/50 border-transparent hover:border-border/50 transition-all duration-200"
              >
                <CardContent className="p-3">
                  <div className="flex items-start gap-3">
                    {/* Citation number - sleek circular badge */}
                    <span className="flex-shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary text-xs flex items-center justify-center font-semibold group-hover:bg-primary group-hover:text-primary-foreground transition-colors">
                      {index + 1}
                    </span>
                    
                    <div className="flex-1 min-w-0 space-y-1.5">
                      {/* Header row */}
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-sm font-medium truncate flex items-center gap-1.5">
                          <FileText className="h-3.5 w-3.5 text-muted-foreground" />
                          {/* Use file_path filename or document ID as fallback */}
                          {chunks[0]?.file_path 
                            ? chunks[0].file_path.split('/').pop() 
                            : `Document ${docId.slice(0, 8)}`}
                        </span>
                        <div className="flex items-center gap-2 flex-shrink-0">
                          <span className={`text-xs font-semibold ${scoreColor}`}>
                            {Math.round(avgScore * 100)}%
                          </span>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                            onClick={() => onDocumentClick?.(docId)}
                            aria-label="Open document in new window"
                          >
                            <ExternalLink className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </div>
                      
                      {/* Document ID hint */}
                      <p className="text-[10px] text-muted-foreground/70 font-mono">
                        {docId.slice(0, 16)}...
                      </p>
                      
                      {/* Content preview */}
                      <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
                        "{chunks[0]?.content.slice(0, 120)}..."
                      </p>
                      
                      {/* Chunk count badge */}
                      {chunks.length > 1 && (
                        <Badge variant="outline" className="text-[10px] h-5">
                          {chunks.length} passages
                        </Badge>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      </ScrollArea>
      
      {entries.length > 3 && !showAll && (
        <Button
          variant="ghost"
          size="sm"
          className="w-full text-xs text-muted-foreground hover:text-foreground gap-1"
          onClick={() => setShowAll(true)}
        >
          Show {entries.length - 3} more sources
          <ChevronDown className="h-3 w-3" />
        </Button>
      )}
    </div>
  );
};

// Knowledge Tab Component
const KnowledgeTab = ({
  entities,
  relationships,
  onEntityClick,
  onDocumentClick,
}: {
  entities: QueryContext['entities'];
  relationships: QueryContext['relationships'];
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (documentId: string) => void;
}) => {
  const [showAllEntities, setShowAllEntities] = useState(false);
  const visibleEntities = showAllEntities ? entities : entities?.slice(0, 12);
  
  const hasContent = (entities && entities.length > 0) || (relationships && relationships.length > 0);
  
  if (!hasContent) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
        <Brain className="h-8 w-8 mb-2 opacity-50" />
        <p className="text-sm">No knowledge extracted</p>
      </div>
    );
  }

  return (
    <ScrollArea className="max-h-[300px]">
      <div className="space-y-5 pr-2">
        {/* Entities */}
        {entities && entities.length > 0 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <Sparkles className="h-3.5 w-3.5 text-primary" />
              <h4 className="text-xs font-semibold text-foreground">Key Topics</h4>
              <Badge variant="secondary" className="text-[10px] h-4 px-1.5">
                {entities.length}
              </Badge>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {visibleEntities?.map((entity) => (
                <HoverCard key={entity.id} openDelay={300}>
                  <HoverCardTrigger asChild>
                    <Badge
                      variant="secondary"
                      className="cursor-pointer hover:bg-primary/15 hover:text-primary hover:border-primary/30 border border-transparent transition-all duration-200 text-xs py-1 px-2.5"
                      onClick={() => onEntityClick?.(entity.id)}
                    >
                      {entity.label}
                    </Badge>
                  </HoverCardTrigger>
                  <HoverCardContent className="w-72" align="start">
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <p className="font-medium">{entity.label}</p>
                        <Badge variant="outline" className="text-[10px]">
                          {Math.round(entity.relevance * 100)}% match
                        </Badge>
                      </div>
                      {(entity.source_file_path || entity.source_document_id) && (
                        <button
                          onClick={() => entity.source_document_id && onDocumentClick?.(entity.source_document_id)}
                          className="text-xs text-primary hover:underline flex items-center gap-1"
                        >
                          <FileText className="h-3 w-3" />
                          View source document
                          <ExternalLink className="h-2.5 w-2.5" />
                        </button>
                      )}
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full text-xs h-7"
                        onClick={() => onEntityClick?.(entity.id)}
                      >
                        <Network className="h-3 w-3 mr-1.5" />
                        Explore in graph
                      </Button>
                    </div>
                  </HoverCardContent>
                </HoverCard>
              ))}
              {entities.length > 12 && !showAllEntities && (
                <Badge
                  variant="outline"
                  className="cursor-pointer hover:bg-muted text-xs py-1 px-2.5"
                  onClick={() => setShowAllEntities(true)}
                >
                  +{entities.length - 12} more
                </Badge>
              )}
            </div>
          </div>
        )}
        
        {/* Relationships */}
        {relationships && relationships.length > 0 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <Network className="h-3.5 w-3.5 text-primary" />
              <h4 className="text-xs font-semibold text-foreground">Connections</h4>
              <Badge variant="secondary" className="text-[10px] h-4 px-1.5">
                {relationships.length}
              </Badge>
            </div>
            <div className="space-y-1">
              {relationships.slice(0, 6).map((rel, idx) => (
                <HoverCard key={idx} openDelay={300}>
                  <HoverCardTrigger asChild>
                    <div
                      className="flex items-center gap-1.5 text-xs p-2 rounded-md hover:bg-muted/60 transition-colors cursor-pointer group"
                    >
                      <span
                        className="font-medium hover:text-primary cursor-pointer truncate max-w-[100px]"
                        onClick={(e) => {
                          e.stopPropagation();
                          onEntityClick?.(rel.source);
                        }}
                      >
                        {rel.source}
                      </span>
                      <span className="text-primary/60 group-hover:text-primary transition-colors">→</span>
                      <Badge variant="outline" className="text-[10px] px-1.5 h-4 font-normal">
                        {rel.type.toLowerCase().replace(/_/g, ' ')}
                      </Badge>
                      <span className="text-primary/60 group-hover:text-primary transition-colors">→</span>
                      <span
                        className="font-medium hover:text-primary cursor-pointer truncate max-w-[100px]"
                        onClick={(e) => {
                          e.stopPropagation();
                          onEntityClick?.(rel.target);
                        }}
                      >
                        {rel.target}
                      </span>
                      <span className="ml-auto text-[10px] text-muted-foreground">
                        {Math.round(rel.relevance * 100)}%
                      </span>
                    </div>
                  </HoverCardTrigger>
                  <HoverCardContent className="w-64" align="start">
                    <div className="space-y-2">
                      <p className="text-sm font-medium">{rel.source} → {rel.target}</p>
                      <Badge variant="secondary" className="text-[10px]">{rel.type}</Badge>
                      {(rel.source_file_path || rel.source_document_id) && (
                        <button
                          onClick={() => rel.source_document_id && onDocumentClick?.(rel.source_document_id)}
                          className="text-xs text-primary hover:underline flex items-center gap-1"
                        >
                          <FileText className="h-3 w-3" />
                          View source
                          <ExternalLink className="h-2.5 w-2.5" />
                        </button>
                      )}
                    </div>
                  </HoverCardContent>
                </HoverCard>
              ))}
              {relationships.length > 6 && (
                <p className="text-xs text-muted-foreground pl-2 pt-1">
                  +{relationships.length - 6} more connections
                </p>
              )}
            </div>
          </div>
        )}
      </div>
    </ScrollArea>
  );
};

// Explore Tab Component
const ExploreTab = ({
  entityCount,
  relationshipCount,
  onExploreGraph,
}: {
  entityCount: number;
  relationshipCount: number;
  onExploreGraph?: () => void;
}) => (
  <div className="flex flex-col items-center justify-center py-8 space-y-4">
    <div className="relative">
      <div className="w-20 h-20 rounded-full bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center">
        <Network className="h-8 w-8 text-primary" />
      </div>
      <div className="absolute -top-1 -right-1 w-6 h-6 rounded-full bg-primary text-primary-foreground text-[10px] font-semibold flex items-center justify-center">
        {entityCount}
      </div>
    </div>
    <div className="text-center space-y-1">
      <p className="text-sm font-semibold">Explore Knowledge Graph</p>
      <p className="text-xs text-muted-foreground">
        {entityCount} topics · {relationshipCount} connections
      </p>
    </div>
    <Button 
      onClick={onExploreGraph} 
      className="gap-2"
      size="sm"
    >
      <Network className="h-4 w-4" />
      Open Graph Explorer
    </Button>
  </div>
);

// ─────────────────────────────────────────────────────────────────────────────
// Main Component
// ─────────────────────────────────────────────────────────────────────────────

export function SourceCitations({
  context,
  onEntityClick,
  onDocumentClick,
  onExploreGraph,
}: SourceCitationsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Memoized calculations
  const hasChunks = context.chunks && context.chunks.length > 0;
  const hasEntities = context.entities && context.entities.length > 0;
  const hasRelationships = context.relationships && context.relationships.length > 0;

  const chunksByDocument = useMemo(() => 
    context.chunks?.reduce((acc, chunk) => {
      if (!acc[chunk.document_id]) {
        acc[chunk.document_id] = [];
      }
      acc[chunk.document_id].push(chunk);
      return acc;
    }, {} as Record<string, NonNullable<typeof context.chunks>>) || {},
    [context.chunks]
  );

  const confidence = useMemo(() => calculateConfidence(context), [context]);
  const { label: confidenceLabel, color: confidenceColor } = getConfidenceLabel(confidence);
  
  const sourceCount = context.chunks?.length || 0;
  const topicCount = context.entities?.length || 0;

  if (!hasChunks && !hasEntities && !hasRelationships) {
    return null;
  }

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="w-full flex items-center justify-between text-muted-foreground hover:text-foreground py-2 h-auto"
          aria-expanded={isExpanded}
          aria-label={`Source citations: ${sourceCount} sources, ${topicCount} topics, ${confidenceLabel} confidence`}
        >
          <span className="flex items-center gap-2">
            <BookOpen className="h-4 w-4" />
            <span className="text-xs font-medium">
              {sourceCount} Source{sourceCount !== 1 ? 's' : ''} · {topicCount} Topic{topicCount !== 1 ? 's' : ''}
            </span>
            <span className={`text-xs flex items-center gap-1.5 ${confidenceColor}`}>
              <ConfidenceDots score={confidence} />
              <span className="font-semibold hidden sm:inline">
                {confidenceLabel} ({Math.round(confidence * 100)}%)
              </span>
              <span className="font-semibold sm:hidden">
                {Math.round(confidence * 100)}%
              </span>
            </span>
          </span>
          {isExpanded ? (
            <ChevronUp className="h-4 w-4 ml-2 flex-shrink-0" />
          ) : (
            <ChevronDown className="h-4 w-4 ml-2 flex-shrink-0" />
          )}
        </Button>
      </CollapsibleTrigger>

      <CollapsibleContent className="mt-2 animate-in fade-in-0 slide-in-from-top-1 duration-200">
        <Card className="border-muted/50 shadow-sm">
          <CardContent className="p-3">
            <Tabs defaultValue="documents" className="w-full">
              <TabsList className="grid w-full grid-cols-3 h-9 mb-3">
                <TabsTrigger value="documents" className="text-xs gap-1.5 data-[state=active]:bg-background">
                  <FileText className="h-3 w-3" />
                  <span className="hidden sm:inline">Documents</span>
                  <span className="sm:hidden">Docs</span>
                </TabsTrigger>
                <TabsTrigger value="knowledge" className="text-xs gap-1.5 data-[state=active]:bg-background">
                  <Brain className="h-3 w-3" />
                  <span className="hidden sm:inline">Knowledge</span>
                  <span className="sm:hidden">Info</span>
                </TabsTrigger>
                <TabsTrigger value="explore" className="text-xs gap-1.5 data-[state=active]:bg-background">
                  <Network className="h-3 w-3" />
                  <span className="hidden sm:inline">Explore</span>
                  <span className="sm:hidden">Graph</span>
                </TabsTrigger>
              </TabsList>
              
              <TabsContent value="documents" className="mt-0 focus-visible:outline-none">
                <DocumentsTab 
                  chunksByDocument={chunksByDocument}
                  onDocumentClick={onDocumentClick}
                />
              </TabsContent>
              
              <TabsContent value="knowledge" className="mt-0 focus-visible:outline-none">
                <KnowledgeTab
                  entities={context.entities}
                  relationships={context.relationships}
                  onEntityClick={onEntityClick}
                  onDocumentClick={onDocumentClick}
                />
              </TabsContent>
              
              <TabsContent value="explore" className="mt-0 focus-visible:outline-none">
                <ExploreTab
                  entityCount={context.entities?.length || 0}
                  relationshipCount={context.relationships?.length || 0}
                  onExploreGraph={onExploreGraph}
                />
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
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
