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
import { Brain, Database, Download, FileText, GitBranch, Network, Settings } from 'lucide-react';
import { CollapsibleSection } from './collapsible-section';
import { DocumentHierarchyTree } from './document-hierarchy-tree';
import { EnhancedMetadata } from './enhanced-metadata';
import { EntityRelationStats } from './entity-relation-stats';
import { KeyStats } from './key-stats';
import { LineageExport } from './lineage-export';
import { LineageTree } from './lineage-tree';
import { ProcessingDetails } from './processing-details';
import { SourceInfoGrid } from './source-info-grid';

interface MetadataSidebarProps {
  document: Document;
}

export function MetadataSidebar({ document }: MetadataSidebarProps) {
  return (
    <div className="h-full flex flex-col border-l bg-background overflow-hidden">
      {/* Fixed Stats Header - Always visible, never compressed */}
      <div className="shrink-0 z-10 bg-background border-b p-4 shadow-sm">
        <KeyStats document={document} />
      </div>

      {/* Scrollable sections - min-h-0 allows flex item to shrink below content height */}
      <ScrollArea className="flex-1 min-h-0" showShadows>
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

          {/* Document Hierarchy Tree (OODA-13): Doc → Chunks → Entities */}
          <CollapsibleSection
            title="Data Hierarchy"
            icon={<GitBranch className="h-4 w-4" />}
          >
            <DocumentHierarchyTree
              documentId={document.id}
              documentName={document.file_name ?? document.title ?? undefined}
            />
          </CollapsibleSection>

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

          {/* Enhanced Metadata from KV Storage (OODA-12) */}
          <CollapsibleSection
            title="Extended Metadata"
            icon={<Database className="h-4 w-4" />}
          >
            <EnhancedMetadata documentId={document.id} />
          </CollapsibleSection>

          {/* Lineage Export (OODA-24): Download lineage as JSON/CSV */}
          <CollapsibleSection
            title="Export Lineage"
            icon={<Download className="h-4 w-4" />}
          >
            <LineageExport documentId={document.id} />
          </CollapsibleSection>
        </div>
      </ScrollArea>
    </div>
  );
}
