/**
 * @module SourceCitations
 * @description Displays source citations for RAG query responses.
 * Shows entities, documents, chunks, and relationships with confidence scores.
 * 
 * @implements UC0203 - Display source citations in response
 * @implements UC0302 - Navigate to source document from citation
 * @implements UC0303 - Explore entity in knowledge graph from citation
 * @implements FEAT0401 - Clickable entity citations with hover preview
 * @implements FEAT0402 - Document deep-links with line numbers
 * @implements FEAT0403 - Confidence score visualization
 * 
 * @enforces BR0104 - Every response shows relevant sources
 * @enforces BR0201 - Entity click syncs with graph panel
 * @enforces BR0402 - Document click opens preview with context
 * 
 * @see {@link specs/025-source-citations-deep-link/01-source-citations-ux-specification.md}
 * @see {@link docs/features.md} FEAT0401-0403
 */
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
  onDocumentClick?: (
    documentId: string, 
    chunkContent?: string, 
    chunkIndex?: number,
    startLine?: number,
    endLine?: number,
    chunkId?: string
  ) => void;
  onExploreGraph?: (entityLabels: string[]) => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

const calculateConfidence = (context: QueryContext): number => {
  // Use chunk scores as primary signal (they're reliable cosine similarities)
  const chunkScores = context.chunks?.map(c => c.score).filter(s => s > 0) || [];
  
  if (chunkScores.length === 0) {
    // No chunks: use entity/relationship relevance, filtering zeros
    const entityScores = context.entities?.map(e => e.relevance).filter(r => r > 0) || [];
    const relScores = context.relationships?.map(r => r.relevance).filter(r => r > 0) || [];
    const allScores = [...entityScores, ...relScores];
    if (allScores.length === 0) return 0.5; // Default medium confidence
    return allScores.reduce((a, b) => a + b, 0) / allScores.length;
  }
  
  // Weighted calculation: max score (60%) + average (30%) + entity bonus (10%)
  const maxScore = Math.max(...chunkScores);
  const avgScore = chunkScores.reduce((a, b) => a + b, 0) / chunkScores.length;
  const entityBonus = Math.min(0.1, (context.entities?.length || 0) * 0.005);
  
  return Math.min(1.0, maxScore * 0.6 + avgScore * 0.3 + entityBonus);
};

/**
 * Get confidence label with NEUTRAL colors (no scary red/orange).
 * 
 * RAG systems typically return scores in 0.2-0.6 range for good matches.
 * Using neutral blues/grays avoids alarming users with "warning" colors.
 * 
 * Thresholds adjusted for realistic RAG similarity scores:
 * - 0.5+: Strong match (primary blue)
 * - 0.3+: Good match (secondary blue)  
 * - 0.2+: Related (neutral gray)
 * - <0.2: Weak (lighter gray)
 */
const getConfidenceLabel = (score: number): { label: string; color: string; bgColor: string } => {
  if (score >= 0.5) return { label: 'Strong', color: 'text-blue-600 dark:text-blue-400', bgColor: 'bg-blue-500' };
  if (score >= 0.3) return { label: 'Good', color: 'text-sky-600 dark:text-sky-400', bgColor: 'bg-sky-500' };
  if (score >= 0.2) return { label: 'Related', color: 'text-slate-600 dark:text-slate-400', bgColor: 'bg-slate-500' };
  return { label: 'Mentioned', color: 'text-slate-500 dark:text-slate-400', bgColor: 'bg-slate-400' };
};

/**
 * Extract a meaningful document title from available data.
 * 
 * Priority:
 * 1. file_path filename (without extension)
 * 2. First markdown heading from content
 * 3. First line of content (truncated)
 * 4. Fallback to "Untitled Document"
 */
const getDocumentTitle = (chunks: NonNullable<QueryContext['chunks']>): string => {
  const chunk = chunks[0];
  if (!chunk) return 'Untitled';
  
  // Priority 1: Extract filename from file_path
  if (chunk.file_path) {
    const filename = chunk.file_path.split('/').pop() || '';
    // Remove common extensions for cleaner display
    const cleanName = filename.replace(/\.(md|txt|pdf|docx?|html?|rst|json|xml)$/i, '');
    if (cleanName.length > 0) {
      // Truncate long filenames
      return cleanName.length > 50 ? cleanName.slice(0, 50) + '...' : cleanName;
    }
  }
  
  // Priority 2: Extract first markdown heading from content
  const titleMatch = chunk.content.match(/^#+\s+(.+)$/m);
  if (titleMatch && titleMatch[1]) {
    const title = titleMatch[1].trim();
    return title.length > 50 ? title.slice(0, 50) + '...' : title;
  }
  
  // Priority 3: Use first meaningful line
  const lines = chunk.content.split('\n').filter(line => line.trim().length > 0);
  if (lines.length > 0) {
    const firstLine = lines[0].trim();
    // Skip if it's just markdown syntax or too short
    if (firstLine.length > 3 && !firstLine.match(/^[-*#=]+$/)) {
      return firstLine.length > 50 ? firstLine.slice(0, 50) + '...' : firstLine;
    }
  }
  
  return 'Untitled Document';
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
  onDocumentClick?: (
    docId: string, 
    chunkContent?: string, 
    chunkIndex?: number,
    startLine?: number,
    endLine?: number,
    chunkId?: string
  ) => void;
}) => {
  // Track which documents have their chunk list expanded beyond the default 3
  const [expandedDocs, setExpandedDocs] = useState<Set<string>>(new Set());
  const entries = Object.entries(chunksByDocument);

  // Normalized sum: score / max(1, globalMax) so each value stays in [0, 1].
  // Handles backends that return unbounded dot-product or hybrid scores (> 1.0).
  const allRawScores = entries.flatMap(([, chunks]) => chunks.map(c => c.score));
  const scoreNormalizer = allRawScores.length > 0 ? Math.max(1.0, ...allRawScores) : 1.0;
  const normalizeScore = (score: number): number => Math.min(1.0, score / scoreNormalizer);

  const toggleDocExpand = (docId: string) => {
    setExpandedDocs(prev => {
      const next = new Set(prev);
      if (next.has(docId)) next.delete(docId); else next.add(docId);
      return next;
    });
  };
  
  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-70 sm:h-90 text-muted-foreground">
        <FileText className="h-8 w-8 mb-2 opacity-50" aria-hidden="true" />
        <p className="text-sm">No source documents</p>
      </div>
    );
  }

  const totalChunks = entries.reduce((acc, [, chunks]) => acc + chunks.length, 0);

  return (
    <div className="space-y-1.5">
      {/* Summary row */}
      <p className="text-[10px] text-muted-foreground px-0.5">
        {entries.length} document{entries.length !== 1 ? 's' : ''}
        {' · '}
        {totalChunks} passage{totalChunks !== 1 ? 's' : ''}
      </p>

      {/* Scrollable document list — fixed height ensures consistent layout across tab switches */}
      <ScrollArea className="h-70 sm:h-83">
        <div className="space-y-2 pr-2" role="list" aria-label="Source documents">
          {entries.map(([docId, chunks], index) => {
            // Normalized sum: mean of per-passage normalized scores → bounded [0, 1]
            const avgScore = chunks.reduce((acc, c) => acc + normalizeScore(c.score), 0) / chunks.length;
            const { color: scoreColor } = getConfidenceLabel(avgScore);
            const isExpanded = expandedDocs.has(docId);
            const visibleChunks = isExpanded ? chunks : chunks.slice(0, 3);
            const hiddenCount = chunks.length - 3;
            
            return (
              <Card 
                key={docId} 
                className="group bg-card border border-border/50 hover:border-border hover:shadow-sm transition-all duration-200"
                role="listitem"
              >
                <CardContent className="p-3">
                  <div className="flex items-start gap-3">
                    {/* Citation index bubble */}
                    <span
                      className="flex-shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary text-xs flex items-center justify-center font-semibold group-hover:bg-primary group-hover:text-primary-foreground transition-colors"
                      aria-hidden="true"
                    >
                      {index + 1}
                    </span>
                    
                    <div className="flex-1 min-w-0 space-y-1.5">
                      {/* Header row: clickable title + score + chunk count */}
                      <div className="flex items-center justify-between gap-2">
                        <button
                          className="text-sm font-semibold flex items-center gap-1.5 hover:text-primary transition-colors text-left max-w-full overflow-hidden text-foreground/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 rounded-sm"
                          onClick={() => onDocumentClick?.(docId, chunks[0]?.content, 0, undefined, undefined, chunks[0]?.chunk_id)}
                          title={`Open: ${getDocumentTitle(chunks)}`}
                          aria-label={`Open document: ${getDocumentTitle(chunks)}`}
                        >
                          <FileText className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" aria-hidden="true" />
                          <span className="truncate">{getDocumentTitle(chunks)}</span>
                        </button>
                        <div className="flex items-center gap-1.5 flex-shrink-0">
                          <span className={`text-xs font-semibold ${scoreColor}`}>
                            {Math.round(avgScore * 100)}%
                          </span>
                          {/* Passage count pill: shows total chunks for this document */}
                          {chunks.length > 1 && (
                            <Badge variant="outline" className="text-[9px] h-4 px-1">
                              {chunks.length}×
                            </Badge>
                          )}
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                            onClick={() => onDocumentClick?.(docId, chunks[0]?.content, 0, undefined, undefined, chunks[0]?.chunk_id)}
                            aria-label={`Open document: ${getDocumentTitle(chunks)}`}
                          >
                            <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
                          </Button>
                        </div>
                      </div>
                      
                      {/* Passages — each is clickable and deep-links via ?chunk=<id> */}
                      <div className="space-y-1.5 mt-2">
                        {visibleChunks.map((chunk, chunkIdx) => (
                          <button
                            key={chunk.chunk_id ?? chunkIdx}
                            onClick={() => onDocumentClick?.(
                              docId,
                              chunk.content,
                              chunk.chunk_index ?? chunkIdx,
                              chunk.start_line,
                              chunk.end_line,
                              chunk.chunk_id
                            )}
                          className="w-full text-left p-2 rounded-md bg-muted/30 hover:bg-yellow-50 dark:hover:bg-yellow-900/20 border border-transparent hover:border-yellow-200 dark:hover:border-yellow-800 transition-colors group/chunk focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
                            title="Click to open and highlight this passage in the document viewer"
                            aria-label={`Open passage ${(chunk.chunk_index ?? chunkIdx) + 1}: ${chunk.content.slice(0, 80)}${chunk.content.length > 80 ? '...' : ''}`}
                          >
                            <div className="flex items-start gap-2">
                              <Badge
                                variant="outline"
                                className="text-[9px] h-4 px-1 flex-shrink-0 mt-0.5"
                              >
                                §{(chunk.chunk_index ?? chunkIdx) + 1}
                              </Badge>
                              <p className="text-xs text-foreground/80 line-clamp-2 flex-1 leading-relaxed break-words overflow-hidden">
                                {chunk.content.slice(0, 200)}{chunk.content.length > 200 ? '…' : ''}
                              </p>
                              <div className="flex flex-col items-end gap-1 flex-shrink-0">
                                <span className={`text-xs font-semibold ${getConfidenceLabel(normalizeScore(chunk.score)).color}`}>
                                  {Math.round(normalizeScore(chunk.score) * 100)}%
                                </span>
                                <ExternalLink className="h-2.5 w-2.5 text-muted-foreground opacity-0 group-hover/chunk:opacity-70 transition-opacity" aria-hidden="true" />
                              </div>
                            </div>
                            {/* Locator: PDF page (deep link) > line range > chunk ID fallback */}
                            {chunk.page_start !== undefined ? (
                              <div className="text-[9px] text-muted-foreground mt-1 pl-6 flex items-center gap-1">
                                <BookOpen className="h-2.5 w-2.5 flex-shrink-0" aria-hidden="true" />
                                <span>p.{chunk.page_start}</span>
                                {chunk.chunk_id && (
                                  <a
                                    href={`/documents/${chunk.document_id}?chunk=${chunk.chunk_id}#page=${chunk.page_start}`}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    onClick={(e) => e.stopPropagation()}
                                    className="ml-1 text-blue-500 hover:text-blue-600 underline"
                                    title={`Open PDF at page ${chunk.page_start}`}
                                    aria-label={`Open document at page ${chunk.page_start}`}
                                  >
                                    Go to page
                                  </a>
                                )}
                              </div>
                            ) : chunk.start_line !== undefined && chunk.end_line !== undefined ? (
                              <div className="text-[9px] text-muted-foreground mt-1 pl-6">
                                L{chunk.start_line}–{chunk.end_line}
                              </div>
                            ) : chunk.chunk_id ? (
                              <div className="text-[9px] text-muted-foreground/50 mt-1 pl-6 font-mono truncate">
                                {chunk.chunk_id.slice(0, 12)}…
                              </div>
                            ) : null}
                          </button>
                        ))}

                        {/* Per-document expand / collapse for docs with many passages */}
                        {hiddenCount > 0 && (
                          <button
                            className="w-full text-[10px] text-muted-foreground hover:text-foreground flex items-center justify-center gap-1 py-1 rounded hover:bg-muted/40 transition-colors"
                            onClick={() => toggleDocExpand(docId)}
                          >
                            {isExpanded ? (
                              <><ChevronUp className="h-3 w-3" />Show less</>
                            ) : (
                              <><ChevronDown className="h-3 w-3" />+{hiddenCount} more passage{hiddenCount !== 1 ? 's' : ''}</>
                            )}
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      </ScrollArea>
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
  onDocumentClick?: (documentId: string, chunkContent?: string, chunkIndex?: number, startLine?: number, endLine?: number, chunkId?: string) => void;
}) => {
  const [showAllEntities, setShowAllEntities] = useState(false);
  const [showAllRelationships, setShowAllRelationships] = useState(false);
  const visibleEntities = showAllEntities ? entities : entities?.slice(0, 12);
  
  const hasContent = (entities && entities.length > 0) || (relationships && relationships.length > 0);
  
  if (!hasContent) {
    return (
      <div className="flex flex-col items-center justify-center h-70 sm:h-90 text-muted-foreground">
        <Brain className="h-8 w-8 mb-2 opacity-50" aria-hidden="true" />
        <p className="text-sm">No knowledge extracted</p>
      </div>
    );
  }

  return (
    <ScrollArea className="h-70 sm:h-90">
      <div className="space-y-5 pr-4">
        {/* Entities */}
        {entities && entities.length > 0 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <Sparkles className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
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
                      <div className="flex items-center justify-between gap-2">
                        <p className="font-medium">{entity.label}</p>
                        <div className="flex items-center gap-1 shrink-0">
                          {entity.entity_type &&
                            entity.entity_type !== "UNKNOWN" && (
                              <Badge variant="secondary" className="text-[10px]">
                                {entity.entity_type.toLowerCase()}
                              </Badge>
                            )}
                          <Badge variant="outline" className="text-[10px]">
                            {Math.round(entity.relevance * 100)}% match
                          </Badge>
                        </div>
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
              {(showAllRelationships ? relationships : relationships?.slice(0, 6))?.map((rel, idx) => (
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
                      {/* Only show score if meaningful (> 0) - graph relationships often have no similarity score */}
                      {rel.relevance > 0.01 && (
                        <span className="ml-auto text-[10px] text-muted-foreground">
                          {Math.round(rel.relevance * 100)}%
                        </span>
                      )}
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
              {/* Expand / collapse relationship list */}
              {relationships && relationships.length > 6 && (
                <button
                  className="w-full text-[10px] text-muted-foreground hover:text-foreground flex items-center justify-center gap-1 py-1 rounded hover:bg-muted/40 transition-colors mt-1"
                  onClick={() => setShowAllRelationships(v => !v)}
                >
                  {showAllRelationships ? (
                    <><ChevronUp className="h-3 w-3" />Show less</>
                  ) : (
                    <><ChevronDown className="h-3 w-3" />+{relationships.length - 6} more connections</>
                  )}
                </button>
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
  entities,
  onExploreGraph,
}: {
  entityCount: number;
  relationshipCount: number;
  entities?: QueryContext['entities'];
  onExploreGraph?: (entityLabels: string[]) => void;
}) => {
  const handleExploreClick = () => {
    const labels = entities?.map(e => e.label) || [];
    onExploreGraph?.(labels);
  };
  
  return (
    <div className="flex flex-col items-center justify-center h-70 sm:h-90 space-y-4">
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
        onClick={handleExploreClick} 
        className="gap-2"
        size="sm"
        aria-label={`Explore graph with ${entityCount} topics and ${relationshipCount} connections`}
      >
        <Network className="h-4 w-4" aria-hidden="true" />
        Open Graph Explorer
      </Button>
    </div>
  );
};

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
    [context]
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
            <BookOpen className="h-4 w-4" aria-hidden="true" />
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
                <TabsTrigger value="documents" className="text-xs gap-1 data-[state=active]:bg-background">
                  <FileText className="h-3 w-3" aria-hidden="true" />
                  <span>Docs</span>
                  {Object.keys(chunksByDocument).length > 0 && (
                    <Badge variant="secondary" className="text-[9px] h-3.5 px-1 ml-0.5 hidden sm:flex">
                      {Object.keys(chunksByDocument).length}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger value="knowledge" className="text-xs gap-1 data-[state=active]:bg-background">
                  <Brain className="h-3 w-3" aria-hidden="true" />
                  <span>Topics</span>
                  {(topicCount + (context.relationships?.length ?? 0)) > 0 && (
                    <Badge variant="secondary" className="text-[9px] h-3.5 px-1 ml-0.5 hidden sm:flex">
                      {topicCount + (context.relationships?.length ?? 0)}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger value="explore" className="text-xs gap-1 data-[state=active]:bg-background">
                  <Network className="h-3 w-3" aria-hidden="true" />
                  <span>Graph</span>
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
                  entities={context.entities}
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
