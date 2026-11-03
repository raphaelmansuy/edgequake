'use client';

/**
 * @module SourceCitations
 * @see {@link specs/025-source-citations-deep-link/01-source-citations-ux-specification.md}
 */
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { calculateAnswerConfidence, getConfidenceLabel } from '@/lib/citations/confidence';
import { chunksByDocument } from '@/lib/citations/group-passages';
import type { OnDocumentClick } from '@/lib/citations/types';
import { useWorkspaceCapabilities } from '@/hooks/use-workspace-capabilities';
import { useSettingsStore } from '@/stores/use-settings-store';
import type { QueryContext } from '@/types';
import { BookOpen, Brain, ChevronDown, ChevronUp, FileText, Network } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { ConfidenceDots } from './confidence-dots';
import { DocumentsTab } from './documents-tab';
import { ExploreTab } from './explore-tab';
import { KnowledgeTab } from './knowledge-tab';

export interface SourceCitationsProps {
  context: QueryContext;
  /**
   * The answer these sources belong to. A refusal must not be labelled "Strong": retrieved
   * volume is not support, and the badge would contradict the sentence beside it.
   */
  answer?: string | null;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: OnDocumentClick;
  onExploreGraph?: (entityLabels: string[]) => void;
  /** Open the panel when true (e.g. from source chip click). */
  defaultOpen?: boolean;
  /** Increment to (re)open the panel and scroll it into view, even if already opened once. */
  openSignal?: number;
}

export function SourceCitations({
  context,
  answer,
  onEntityClick,
  onDocumentClick,
  onExploreGraph,
  defaultOpen = false,
  openSignal = 0,
}: SourceCitationsProps) {
  const { t } = useTranslation();
  // SPEC-180: a workspace without a graph cites passages only: no Topics, no Graph tab.
  const showsGraph = useWorkspaceCapabilities().shows_graph_ui;
  const [isExpanded, setIsExpanded] = useState(defaultOpen);

  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (defaultOpen) setIsExpanded(true);
  }, [defaultOpen]);

  // Chip "+N more" → open + reveal. Runs on mount too (panel mounts after streaming ends).
  useEffect(() => {
    if (openSignal <= 0) return;
    setIsExpanded(true);
    const id = requestAnimationFrame(() =>
      rootRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }),
    );
    return () => cancelAnimationFrame(id);
  }, [openSignal]);
  const fullChunkContent = useSettingsStore(
    (state) => state.querySettings.fullChunkContent ?? false,
  );

  const hasChunks = context.chunks && context.chunks.length > 0;
  const hasEntities = context.entities && context.entities.length > 0;
  const hasRelationships = context.relationships && context.relationships.length > 0;

  const groupedChunks = useMemo(
    () => chunksByDocument(context.chunks),
    [context.chunks],
  );

  const confidence = useMemo(
    () => calculateAnswerConfidence(context, answer),
    [context, answer],
  );
  // `null` means there is nothing to judge: show the band for ordering, but never a number.
  const { labelKey, defaultLabel, color: confidenceColor } = getConfidenceLabel(
    confidence ?? 0,
  );
  const confidenceLabel = t(labelKey, defaultLabel);

  const sourceCount = context.chunks?.length || 0;
  const topicCount = context.entities?.length || 0;
  const docTabCount = Object.keys(groupedChunks).length;
  const knowledgeTabCount = topicCount + (context.relationships?.length ?? 0);

  if (!hasChunks && !hasEntities && !hasRelationships) {
    return null;
  }

  const sourceLabel = t('query.citations.sourceCount', '{{count}} Source', { count: sourceCount });
  const topicLabel = t('query.citations.topicCount', '{{count}} Topic', { count: topicCount });

  return (
    <Collapsible ref={rootRef} open={isExpanded} onOpenChange={setIsExpanded} data-testid="source-citations">
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="w-full flex items-center justify-between text-muted-foreground hover:text-foreground py-2 h-auto"
          aria-expanded={isExpanded}
          aria-label={
            showsGraph
              ? t('query.citations.panelAria', 'Source citations: {{sources}} sources, {{topics}} topics, {{confidence}} confidence', {
                  sources: sourceCount,
                  topics: topicCount,
                  confidence: confidenceLabel,
                })
              : t('query.citations.panelAriaPassages', 'Source citations: {{sources}} sources, {{confidence}} confidence', {
                  sources: sourceCount,
                  confidence: confidenceLabel,
                })
          }
        >
          <span className="flex items-center gap-2">
            <BookOpen className="h-4 w-4" aria-hidden="true" />
            <span className="text-xs font-medium">
              {showsGraph ? `${sourceLabel} · ${topicLabel}` : sourceLabel}
            </span>
            {/*
              WHY nothing is rendered on a refusal: `calculateAnswerConfidence` returns `null`
              when the answer declines, so "Strong" never appears beside "insufficient
              evidence" (found on a 20-image workspace, where breadth was maximal and the
              answer was a refusal). And why no percentage: on a normal workspace the chunk
              `score` is an RRF rank score, so any number derived from it is ~1% even for a
              perfect match. The band label describes relative ordering.
            */}
            {confidence === null ? null : (
              <span
                className={`text-xs flex items-center gap-1.5 ${confidenceColor}`}
                data-testid="source-confidence"
              >
                <ConfidenceDots score={confidence} />
                <span className="font-semibold">{confidenceLabel}</span>
              </span>
            )}
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
            {showsGraph ? (
            <Tabs defaultValue="documents" className="w-full">
              <TabsList className="grid w-full grid-cols-3 h-9 mb-3">
                <TabsTrigger
                  value="documents"
                  className="text-xs gap-1 data-[state=active]:bg-background"
                >
                  <FileText className="h-3 w-3" aria-hidden="true" />
                  <span>{t('query.citations.tabDocs', 'Docs')}</span>
                  {docTabCount > 0 && (
                    <Badge variant="secondary" className="text-xs h-3.5 px-1 ml-0.5 hidden sm:flex">
                      {docTabCount}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger
                  value="knowledge"
                  className="text-xs gap-1 data-[state=active]:bg-background"
                >
                  <Brain className="h-3 w-3" aria-hidden="true" />
                  <span>{t('query.citations.tabTopics', 'Topics')}</span>
                  {knowledgeTabCount > 0 && (
                    <Badge variant="secondary" className="text-xs h-3.5 px-1 ml-0.5 hidden sm:flex">
                      {knowledgeTabCount}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger
                  value="explore"
                  className="text-xs gap-1 data-[state=active]:bg-background"
                >
                  <Network className="h-3 w-3" aria-hidden="true" />
                  <span>{t('query.citations.tabGraph', 'Graph')}</span>
                </TabsTrigger>
              </TabsList>

              <TabsContent value="documents" className="mt-0 focus-visible:outline-none">
                <DocumentsTab
                  chunksByDocument={groupedChunks}
                  fullChunkContent={fullChunkContent}
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
            ) : (
              <DocumentsTab
                chunksByDocument={groupedChunks}
                fullChunkContent={fullChunkContent}
                onDocumentClick={onDocumentClick}
              />
            )}
          </CardContent>
        </Card>
      </CollapsibleContent>
    </Collapsible>
  );
}
