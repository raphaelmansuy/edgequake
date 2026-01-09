/**
 * @fileoverview Smart metadata sidebar with collapsible sections
 *
 * @implements FEAT1074 - Document metadata display
 * @implements FEAT1075 - Collapsible section organization
 *
 * @see UC1505 - User views document metadata
 * @see UC1506 - User expands/collapses metadata sections
 *
 * @enforces BR1074 - Sticky key stats header
 * @enforces BR1075 - Scrollable section content
 */
// Smart metadata sidebar with collapsible sections
'use client';

import { ScrollArea } from '@/components/ui/scroll-area';
import type { Document } from '@/types';
import { Brain, FileText, Network, Settings } from 'lucide-react';
import { CollapsibleSection } from './collapsible-section';
import { EntityRelationStats } from './entity-relation-stats';
import { KeyStats } from './key-stats';
import { LineageTree } from './lineage-tree';
import { ProcessingDetails } from './processing-details';
import { SourceInfoGrid } from './source-info-grid';

interface MetadataSidebarProps {
  document: Document;
}

export function MetadataSidebar({ document }: MetadataSidebarProps) {
  return (
    <div className="h-full flex flex-col border-l bg-background">
      {/* Sticky Stats - Always visible */}
      <div className="sticky top-0 z-10 bg-background border-b p-4 shadow-sm">
        <KeyStats document={document} />
      </div>

      {/* Scrollable sections */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {/* Extraction Lineage */}
          {document.lineage && (
            <CollapsibleSection
              title="Extraction Lineage"
              icon={<Brain className="h-4 w-4" />}
              defaultOpen
            >
              <LineageTree lineage={document.lineage} />
            </CollapsibleSection>
          )}

          {/* Entity & Relationships */}
          {(document.entity_count !== undefined || document.relationship_count !== undefined) && (
            <CollapsibleSection
              title="Knowledge Graph"
              icon={<Network className="h-4 w-4" />}
              defaultOpen
            >
              <EntityRelationStats
                entities={document.entity_count}
                relationships={document.relationship_count}
                documentId={document.id}
              />
            </CollapsibleSection>
          )}

          {/* Source Information */}
          <CollapsibleSection
            title="Source Details"
            icon={<FileText className="h-4 w-4" />}
          >
            <SourceInfoGrid document={document} />
          </CollapsibleSection>

          {/* Processing Details */}
          {document.lineage && (
            <CollapsibleSection
              title="Processing Info"
              icon={<Settings className="h-4 w-4" />}
            >
              <ProcessingDetails lineage={document.lineage} />
            </CollapsibleSection>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
