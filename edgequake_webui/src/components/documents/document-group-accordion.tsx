'use client';

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { ChevronDown, ChevronRight, Folder } from 'lucide-react';
import { memo, useState } from 'react';
import { DocumentTableRow } from './document-table-row';
import type { DocumentTableRowProps } from './document-table-row';

/**
 * Handlers passed through to each DocumentTableRow — all row actions.
 */
export interface DocumentGroupRowHandlers
  extends Pick<
    DocumentTableRowProps,
    | 'onSelect'
    | 'onClick'
    | 'onDoubleClick'
    | 'onViewDetails'
    | 'onViewInGraph'
    | 'onViewPdf'
    | 'onRetry'
    | 'onEnrich'
    | 'onCancel'
    | 'onDelete'
    | 'isRetrying'
    | 'isCancelling'
  > {}

export interface DocumentGroupAccordionProps extends DocumentGroupRowHandlers {
  /** Topic label for this group */
  topic: string;
  /** Documents in this group */
  documents: Document[];
  /** Currently selected document IDs */
  selectedIds: Set<string>;
  /** Currently active/previewed document */
  selectedDocument: Document | null;
  /** Current search query (for row highlight) */
  searchQuery: string;
}

/**
 * Collapsible accordion group for documents sharing the same topic.
 * Collapsed by default. "Tanpa Topic" label rendered muted.
 */
export const DocumentGroupAccordion = memo(function DocumentGroupAccordion({
  topic,
  documents,
  selectedIds,
  selectedDocument,
  searchQuery,
  onSelect,
  onClick,
  onDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onEnrich,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
}: DocumentGroupAccordionProps) {
  const [open, setOpen] = useState(false);
  const isNoTopic = topic === 'Tanpa Topic';

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className="w-full flex items-center gap-2 px-4 py-2 bg-muted/30 hover:bg-muted/50 border-b text-left transition-colors"
          aria-expanded={open}
        >
          {open ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
          <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span
            className={cn(
              'text-sm font-medium',
              isNoTopic ? 'text-muted-foreground italic' : 'text-foreground',
            )}
          >
            {topic}
          </span>
          <span className="ml-1.5 text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
            {documents.length}
          </span>
          {/* Show union of all tags in this group */}
          {(() => {
            const allTags = Array.from(
              new Set(
                documents.flatMap((d) => d.enrichment_tags ?? [])
              )
            ).slice(0, 5);
            return allTags.length > 0 ? (
              <div className="ml-2 flex flex-wrap gap-1">
                {allTags.map((tag) => (
                  <span
                    key={tag}
                    className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-secondary/70 text-secondary-foreground"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            ) : null;
          })()}
        </button>
      </CollapsibleTrigger>

      <CollapsibleContent>
        {documents.map((doc, index) => (
          <DocumentTableRow
            key={doc.id}
            doc={doc}
            index={index}
            isSelected={selectedIds.has(doc.id)}
            isActive={selectedDocument?.id === doc.id}
            searchQuery={searchQuery}
            onSelect={onSelect}
            onClick={onClick}
            onDoubleClick={onDoubleClick}
            onViewDetails={onViewDetails}
            onViewInGraph={onViewInGraph}
            onViewPdf={onViewPdf}
            onRetry={onRetry}
            onEnrich={onEnrich}
            onCancel={onCancel}
            onDelete={onDelete}
            isRetrying={isRetrying}
            isCancelling={isCancelling}
          />
        ))}
      </CollapsibleContent>
    </Collapsible>
  );
});
