/**
 * @fileoverview Document hierarchy tree showing Document → Chunks → Entities
 *
 * WHY: The existing LineageTree shows pipeline steps (upload, extract, map, index)
 * but not the actual data hierarchy. This component shows the real structure:
 * which chunks were created, which entities were extracted from each chunk,
 * enabling source traceability.
 *
 * @implements FEAT1088 - Document hierarchy tree visualization
 * @implements F8 - PDF → Document → Chunk → Entity chain traceable
 *
 * @see UC1519 - User views document-to-entity hierarchy
 * @enforces BR1088 - Collapsible tree with entity counts per chunk
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { useDocumentLineage } from '@/hooks/use-lineage';
import { cn } from '@/lib/utils';
import type { ChunkLineage, EntityLineage } from '@/types/lineage';
import {
  ChevronDown,
  ChevronRight,
  FileText,
  Layers,
  Loader2,
  Tag,
} from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';

interface DocumentHierarchyTreeProps {
  documentId: string;
  documentName?: string;
}

export function DocumentHierarchyTree({
  documentId,
  documentName,
}: DocumentHierarchyTreeProps) {
  const { data: lineage, isLoading, error } = useDocumentLineage(documentId);

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground p-2">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        Loading hierarchy...
      </div>
    );
  }

  if (error || !lineage) {
    return (
      <p className="text-xs text-muted-foreground p-2">
        Hierarchy data not available
      </p>
    );
  }

  // Build entity lookup: chunk_id → entities
  const entityByChunk = useMemo(() => {
    const map = new Map<string, EntityLineage[]>();
    for (const entity of lineage.entities ?? []) {
      for (const chunkId of entity.source_chunks ?? []) {
        const list = map.get(chunkId) ?? [];
        list.push(entity);
        map.set(chunkId, list);
      }
    }
    return map;
  }, [lineage.entities]);

  const chunks = lineage.chunks ?? [];
  const totalEntities = lineage.entities?.length ?? 0;

  return (
    <div className="space-y-1">
      {/* Document root node */}
      <TreeNode
        icon={<FileText className="h-3.5 w-3.5" />}
        label={documentName ?? lineage.document_name ?? documentId.slice(0, 8)}
        badge={`${chunks.length} chunks • ${totalEntities} entities`}
        defaultOpen
        depth={0}
      >
        {chunks.length === 0 ? (
          <p className="text-xs text-muted-foreground pl-6 py-1">
            No chunks extracted yet
          </p>
        ) : (
          chunks.map((chunk) => (
            <ChunkTreeNode
              key={chunk.chunk_id}
              chunk={chunk}
              entities={entityByChunk.get(chunk.chunk_id) ?? []}
              depth={1}
            />
          ))
        )}
      </TreeNode>
    </div>
  );
}

// ============================================================================
// Chunk tree node
// ============================================================================

interface ChunkTreeNodeProps {
  chunk: ChunkLineage;
  entities: EntityLineage[];
  depth: number;
}

function ChunkTreeNode({ chunk, entities, depth }: ChunkTreeNodeProps) {
  const lineInfo = chunk.start_line
    ? `L${chunk.start_line}–${chunk.end_line ?? '?'}`
    : `#${chunk.chunk_index ?? chunk.index}`;

  return (
    <TreeNode
      icon={<Layers className="h-3 w-3" />}
      label={`Chunk ${chunk.chunk_index ?? chunk.index}`}
      badge={`${lineInfo} • ${chunk.token_count} tok • ${entities.length} ent`}
      depth={depth}
    >
      {entities.length === 0 ? (
        <p className="text-xs text-muted-foreground pl-6 py-0.5">
          No entities
        </p>
      ) : (
        entities.map((ent) => (
          <EntityLeafNode key={ent.id ?? ent.name} entity={ent} depth={depth + 1} />
        ))
      )}
    </TreeNode>
  );
}

// ============================================================================
// Entity leaf node
// ============================================================================

interface EntityLeafNodeProps {
  entity: EntityLineage;
  depth: number;
}

function EntityLeafNode({ entity, depth }: EntityLeafNodeProps) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 py-1 px-2 rounded text-xs hover:bg-muted/40 transition-colors'
      )}
      style={{ paddingLeft: `${(depth + 1) * 16}px` }}
    >
      <Tag className="h-3 w-3 shrink-0 text-muted-foreground" />
      <span className="font-medium truncate" title={entity.name}>
        {entity.name}
      </span>
      <Badge variant="outline" className="text-[10px] px-1.5 py-0 shrink-0">
        {entity.entity_type}
      </Badge>
      {entity.extraction_count > 1 && (
        <Badge variant="secondary" className="text-[10px] px-1.5 py-0 shrink-0">
          ×{entity.extraction_count}
        </Badge>
      )}
    </div>
  );
}

// ============================================================================
// Generic tree node (collapsible)
// ============================================================================

interface TreeNodeProps {
  icon: React.ReactNode;
  label: string;
  badge?: string;
  defaultOpen?: boolean;
  depth: number;
  children?: React.ReactNode;
}

function TreeNode({
  icon,
  label,
  badge,
  defaultOpen = false,
  depth,
  children,
}: TreeNodeProps) {
  const [open, setOpen] = useState(defaultOpen);
  const toggle = useCallback(() => setOpen((p) => !p), []);

  return (
    <div>
      <button
        type="button"
        onClick={toggle}
        className={cn(
          'flex items-center gap-1.5 w-full text-left py-1.5 px-2 rounded text-sm',
          'hover:bg-muted/50 transition-colors'
        )}
        style={{ paddingLeft: `${depth * 16}px` }}
      >
        {open ? (
          <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        )}
        <span className="shrink-0">{icon}</span>
        <span className="font-medium truncate">{label}</span>
        {badge && (
          <span className="text-xs text-muted-foreground ml-auto shrink-0">
            {badge}
          </span>
        )}
      </button>
      {open && <div>{children}</div>}
    </div>
  );
}
