# WebUI Specification: Lineage Visualization

> Document ID: WEBUI-006
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Overview](#1-overview)
2. [Lineage Data Model](#2-lineage-data-model)
3. [Visualization Components](#3-visualization-components)
4. [Interactive Features](#4-interactive-features)
5. [Graph Layout Algorithm](#5-graph-layout-algorithm)
6. [Performance Optimization](#6-performance-optimization)
7. [Accessibility](#7-accessibility)

---

## 1. Overview

### 1.1 Purpose

This document specifies the lineage visualization system that enables users to trace how documents are transformed into knowledge graph elements. The visualization shows the complete provenance chain: Document → Chunks → Entities/Relationships.

### 1.2 Key Visualization Modes

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       LINEAGE VISUALIZATION MODES                           │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│   TREE VIEW     │   │   GRAPH VIEW    │   │   TABLE VIEW    │
│                 │   │                 │   │                 │
│  📄 Document    │   │     ┌───┐       │   │ ID│Chunk│Entity │
│   ├─📦 Chunk 1  │   │     │Doc│       │   │ ──┼─────┼────── │
│   │  ├─👤 Ent1  │   │    /│   │\      │   │ 1 │ C1  │ Ent1  │
│   │  └─👤 Ent2  │   │  ┌─┘ └──┘└─┐    │   │ 2 │ C1  │ Ent2  │
│   └─📦 Chunk 2  │   │ C1   C2   C3    │   │ 3 │ C2  │ Ent3  │
│      └─👤 Ent3  │   │  |    |    |    │   │...│ ... │ ...   │
│                 │   │ E1  E2,E3  E4   │   │                 │
└─────────────────┘   └─────────────────┘   └─────────────────┘
  Hierarchical          Force-directed        Flat/Searchable
```

### 1.3 Requirements

| Requirement | Description |
|-------------|-------------|
| REQ-LIN-001 | Tree view with expandable/collapsible nodes |
| REQ-LIN-002 | Graph view with force-directed layout |
| REQ-LIN-003 | Click-through to chunk and entity details |
| REQ-LIN-004 | Search/filter by entity name or type |
| REQ-LIN-005 | Highlight provenance path on hover |
| REQ-LIN-006 | Export lineage as JSON/SVG |
| REQ-LIN-007 | Support 1000+ nodes with virtualization |

---

## 2. Lineage Data Model

### 2.1 API Response Types

```typescript
// Types from 12-webui-api-integration.md

interface DocumentLineageResponse {
  document: {
    id: string;
    name: string;
    status: DocumentStatus;
    metadata: DocumentMetadata;
  };
  chunks: ChunkLineage[];
  entities: EntityLineage[];
  relationships: RelationshipLineage[];
  statistics: LineageStatistics;
}

interface ChunkLineage {
  id: string;
  index: number;
  content_preview: string;
  token_count: number;
  char_range: { start: number; end: number };
  extracted_entities: string[];  // Entity IDs
  extracted_relationships: string[];  // Relationship IDs
  extraction_metadata: {
    model: string;
    duration_ms: number;
    prompt_tokens: number;
    completion_tokens: number;
    cost_usd: number;
    cached: boolean;
  };
}

interface EntityLineage {
  id: string;
  name: string;
  entity_type: string;
  description: string;
  source_chunks: string[];  // Chunk IDs
  merged_from?: string[];   // Original entity names before merge
  extraction_count: number;
  confidence?: number;
}

interface RelationshipLineage {
  id: string;
  source_entity: string;
  target_entity: string;
  relation_type: string;
  description: string;
  source_chunks: string[];
  weight: number;
}
```

### 2.2 Graph Node Types

```typescript
// Graph visualization node types

type LineageNode =
  | DocumentNode
  | ChunkNode
  | EntityNode
  | RelationshipNode;

interface DocumentNode {
  type: 'document';
  id: string;
  label: string;
  status: DocumentStatus;
  chunkCount: number;
  entityCount: number;
}

interface ChunkNode {
  type: 'chunk';
  id: string;
  index: number;
  label: string;  // "Chunk 1"
  preview: string;
  tokenCount: number;
  entityCount: number;
  cached: boolean;
}

interface EntityNode {
  type: 'entity';
  id: string;
  name: string;
  entityType: string;
  sourceCount: number;
  merged: boolean;
  confidence?: number;
}

interface RelationshipNode {
  type: 'relationship';
  id: string;
  label: string;
  sourceEntity: string;
  targetEntity: string;
  weight: number;
}

// Graph edges
interface LineageEdge {
  id: string;
  source: string;
  target: string;
  type: 'contains' | 'extracted' | 'merged' | 'relates';
  weight?: number;
}
```

---

## 3. Visualization Components

### 3.1 LineageExplorer (Main Container)

```tsx
// src/components/lineage/lineage-explorer.tsx

interface LineageExplorerProps {
  documentId: string;
  initialView?: 'tree' | 'graph' | 'table';
  onChunkClick?: (chunkId: string) => void;
  onEntityClick?: (entityId: string) => void;
  className?: string;
}

export function LineageExplorer({
  documentId,
  initialView = 'tree',
  onChunkClick,
  onEntityClick,
  className,
}: LineageExplorerProps) {
  const [view, setView] = useState(initialView);
  const [filter, setFilter] = useState('');
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  
  const { data: lineage, isLoading, error } = useDocumentLineage(documentId);
  
  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Toolbar */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex gap-2">
          <Tabs value={view} onValueChange={setView}>
            <TabsList>
              <TabsTrigger value="tree">
                <TreeIcon className="h-4 w-4 mr-1" />
                Tree
              </TabsTrigger>
              <TabsTrigger value="graph">
                <NetworkIcon className="h-4 w-4 mr-1" />
                Graph
              </TabsTrigger>
              <TabsTrigger value="table">
                <TableIcon className="h-4 w-4 mr-1" />
                Table
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
        
        <div className="flex gap-2">
          <Input
            placeholder="Filter entities..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-64"
          />
          <ExportButton lineage={lineage} />
        </div>
      </div>
      
      {/* View Content */}
      <div className="flex-1 overflow-hidden">
        {view === 'tree' && (
          <LineageTreeView
            lineage={lineage}
            filter={filter}
            selectedNode={selectedNode}
            onNodeSelect={setSelectedNode}
            onChunkClick={onChunkClick}
            onEntityClick={onEntityClick}
          />
        )}
        {view === 'graph' && (
          <LineageGraphView
            lineage={lineage}
            filter={filter}
            selectedNode={selectedNode}
            onNodeSelect={setSelectedNode}
            onChunkClick={onChunkClick}
            onEntityClick={onEntityClick}
          />
        )}
        {view === 'table' && (
          <LineageTableView
            lineage={lineage}
            filter={filter}
            onChunkClick={onChunkClick}
            onEntityClick={onEntityClick}
          />
        )}
      </div>
      
      {/* Statistics Footer */}
      <LineageStatisticsBar statistics={lineage?.statistics} />
    </div>
  );
}
```

### 3.2 Tree View Component

```tsx
// src/components/lineage/lineage-tree-view.tsx

interface LineageTreeViewProps {
  lineage: DocumentLineageResponse | null;
  filter: string;
  selectedNode: string | null;
  onNodeSelect: (nodeId: string) => void;
  onChunkClick?: (chunkId: string) => void;
  onEntityClick?: (entityId: string) => void;
}

export function LineageTreeView({
  lineage,
  filter,
  selectedNode,
  onNodeSelect,
  onChunkClick,
  onEntityClick,
}: LineageTreeViewProps) {
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set());
  
  if (!lineage) return <Skeleton />;
  
  const filteredChunks = useMemo(() => {
    if (!filter) return lineage.chunks;
    const lowerFilter = filter.toLowerCase();
    return lineage.chunks.filter(chunk => {
      const entities = lineage.entities.filter(e => 
        chunk.extracted_entities.includes(e.id)
      );
      return entities.some(e => 
        e.name.toLowerCase().includes(lowerFilter) ||
        e.entity_type.toLowerCase().includes(lowerFilter)
      );
    });
  }, [lineage, filter]);
  
  const toggleNode = (nodeId: string) => {
    setExpandedNodes(prev => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };
  
  return (
    <div className="p-4 overflow-auto h-full">
      <Tree>
        {/* Document Root */}
        <TreeNode
          icon={<FileIcon />}
          label={lineage.document.name}
          isExpanded={true}
          className="font-medium"
        >
          {/* Chunks */}
          {filteredChunks.map((chunk, index) => (
            <TreeNode
              key={chunk.id}
              icon={<BoxIcon />}
              label={`Chunk ${chunk.index + 1}`}
              badge={`${chunk.extracted_entities.length} entities`}
              isExpanded={expandedNodes.has(chunk.id)}
              onToggle={() => toggleNode(chunk.id)}
              onClick={() => onChunkClick?.(chunk.id)}
              isSelected={selectedNode === chunk.id}
              metadata={
                <ChunkMetadata
                  tokenCount={chunk.token_count}
                  model={chunk.extraction_metadata.model}
                  duration={chunk.extraction_metadata.duration_ms}
                  cached={chunk.extraction_metadata.cached}
                />
              }
            >
              {/* Entities in this chunk */}
              {lineage.entities
                .filter(e => chunk.extracted_entities.includes(e.id))
                .map(entity => (
                  <TreeNode
                    key={entity.id}
                    icon={<EntityIcon type={entity.entity_type} />}
                    label={entity.name}
                    badge={entity.entity_type}
                    onClick={() => onEntityClick?.(entity.id)}
                    isSelected={selectedNode === entity.id}
                    className={entity.merged_from ? 'text-amber-600' : ''}
                  >
                    {entity.merged_from && (
                      <MergedFromIndicator names={entity.merged_from} />
                    )}
                  </TreeNode>
                ))}
            </TreeNode>
          ))}
        </TreeNode>
      </Tree>
    </div>
  );
}
```

**Visual Structure:**

```
┌────────────────────────────────────────────────────────────────────────────┐
│ 📄 research-paper.pdf                                                      │
│ ├─ 📦 Chunk 1                                    [3 entities] │ 1,200 tok │
│ │  ├─ 👤 Dr. Sarah Chen                          PERSON       │ ⚡ cached  │
│ │  ├─ 🏢 Quantum Labs                            ORGANIZATION │           │
│ │  └─ 🔬 Neural Interface Project                PROJECT      │           │
│ ├─ 📦 Chunk 2                                    [2 entities] │ 1,180 tok │
│ │  ├─ 🏢 MIT                                     ORGANIZATION │           │
│ │  └─ 🏢 Stanford                                ORGANIZATION │           │
│ ├─ 📦 Chunk 3                                    [4 entities] │ 1,050 tok │
│ │  ├─ 👤 MARCUS_REEVES ⚠ merged                  PERSON       │           │
│ │  │     └─ Merged from: Marcus Reeves, M. Reeves             │           │
│ │  ├─ 💡 Quantum Entanglement                    CONCEPT      │           │
│ │  ├─ 📊 2023 Study                              PUBLICATION  │           │
│ │  └─ 🏢 Research Council                        ORGANIZATION │           │
│ └─ ... (7 more chunks)                                                     │
└────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Graph View Component

```tsx
// src/components/lineage/lineage-graph-view.tsx

import { useCallback, useRef, useState } from 'react';
import ReactFlow, {
  Node,
  Edge,
  Controls,
  Background,
  MiniMap,
  useNodesState,
  useEdgesState,
  MarkerType,
} from 'reactflow';
import 'reactflow/dist/style.css';

interface LineageGraphViewProps {
  lineage: DocumentLineageResponse | null;
  filter: string;
  selectedNode: string | null;
  onNodeSelect: (nodeId: string) => void;
  onChunkClick?: (chunkId: string) => void;
  onEntityClick?: (entityId: string) => void;
}

// Custom node types
const nodeTypes = {
  document: DocumentGraphNode,
  chunk: ChunkGraphNode,
  entity: EntityGraphNode,
};

export function LineageGraphView({
  lineage,
  filter,
  selectedNode,
  onNodeSelect,
  onChunkClick,
  onEntityClick,
}: LineageGraphViewProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  
  // Transform lineage data to React Flow format
  useEffect(() => {
    if (!lineage) return;
    
    const { nodes: graphNodes, edges: graphEdges } = transformLineageToGraph(lineage, filter);
    setNodes(graphNodes);
    setEdges(graphEdges);
  }, [lineage, filter, setNodes, setEdges]);
  
  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    onNodeSelect(node.id);
    if (node.type === 'chunk') {
      onChunkClick?.(node.id);
    } else if (node.type === 'entity') {
      onEntityClick?.(node.id);
    }
  }, [onNodeSelect, onChunkClick, onEntityClick]);
  
  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-right"
      >
        <Controls />
        <Background variant="dots" gap={12} size={1} />
        <MiniMap
          nodeColor={(node) => {
            switch (node.type) {
              case 'document': return '#3b82f6';
              case 'chunk': return '#22c55e';
              case 'entity': return '#f59e0b';
              default: return '#9ca3af';
            }
          }}
        />
      </ReactFlow>
    </div>
  );
}

// Transform function
function transformLineageToGraph(
  lineage: DocumentLineageResponse,
  filter: string
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  
  // Document node at center
  nodes.push({
    id: lineage.document.id,
    type: 'document',
    position: { x: 400, y: 50 },
    data: {
      label: lineage.document.name,
      status: lineage.document.status,
      chunkCount: lineage.chunks.length,
    },
  });
  
  // Chunk nodes in a row below document
  const chunkSpacing = 150;
  const startX = 400 - ((lineage.chunks.length - 1) * chunkSpacing) / 2;
  
  lineage.chunks.forEach((chunk, index) => {
    nodes.push({
      id: chunk.id,
      type: 'chunk',
      position: { x: startX + index * chunkSpacing, y: 200 },
      data: {
        index: chunk.index,
        preview: chunk.content_preview,
        entityCount: chunk.extracted_entities.length,
        cached: chunk.extraction_metadata.cached,
      },
    });
    
    // Edge from document to chunk
    edges.push({
      id: `doc-${chunk.id}`,
      source: lineage.document.id,
      target: chunk.id,
      type: 'smoothstep',
      animated: false,
      style: { stroke: '#94a3b8' },
    });
  });
  
  // Entity nodes below chunks
  const entityPositions = new Map<string, { x: number; y: number }>();
  let entityY = 400;
  
  lineage.entities.forEach((entity, index) => {
    // Filter by search term
    if (filter && !entity.name.toLowerCase().includes(filter.toLowerCase())) {
      return;
    }
    
    // Calculate position based on connected chunks
    const connectedChunks = entity.source_chunks;
    const avgX = connectedChunks.reduce((sum, chunkId) => {
      const chunkNode = nodes.find(n => n.id === chunkId);
      return sum + (chunkNode?.position.x ?? 0);
    }, 0) / connectedChunks.length || 400;
    
    nodes.push({
      id: entity.id,
      type: 'entity',
      position: { x: avgX + (index % 3 - 1) * 80, y: entityY + Math.floor(index / 3) * 100 },
      data: {
        name: entity.name,
        entityType: entity.entity_type,
        merged: !!entity.merged_from,
        sourceCount: entity.source_chunks.length,
      },
    });
    
    // Edges from chunks to entity
    entity.source_chunks.forEach(chunkId => {
      edges.push({
        id: `${chunkId}-${entity.id}`,
        source: chunkId,
        target: entity.id,
        type: 'smoothstep',
        animated: false,
        style: { stroke: '#f59e0b', strokeWidth: 2 },
        markerEnd: { type: MarkerType.ArrowClosed },
      });
    });
  });
  
  return { nodes, edges };
}
```

**Visual Structure:**

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                                                            │
│                           ┌─────────────────┐                              │
│                           │  📄 Document     │                              │
│                           │  research.pdf   │                              │
│                           └────────┬────────┘                              │
│                    ┌───────────────┼───────────────┐                       │
│                    │               │               │                       │
│              ┌─────┴─────┐   ┌─────┴─────┐   ┌─────┴─────┐                │
│              │ 📦 Chunk 1 │   │ 📦 Chunk 2 │   │ 📦 Chunk 3 │                │
│              │  3 ent    │   │  2 ent    │   │  4 ent    │                │
│              └─────┬─────┘   └─────┬─────┘   └─────┬─────┘                │
│                    │               │               │                       │
│           ┌────────┼────────┬──────┴──────┬────────┼────────┐             │
│           │        │        │             │        │        │             │
│      ┌────┴────┐ ┌─┴──┐ ┌───┴───┐    ┌───┴───┐ ┌──┴──┐ ┌───┴───┐        │
│      │ 👤 Sarah │ │ 🏢 │ │ 🏢 MIT │    │ 👤 Marc│ │ 💡  │ │ 📊 2023│        │
│      │  Chen   │ │ QL │ │       │    │  ⚠    │ │ QE  │ │ Study │        │
│      └─────────┘ └────┘ └───────┘    └───────┘ └─────┘ └───────┘        │
│                                                                            │
├─────────────────────────────────────────────────── MINIMAP ────────────────┤
│  ┌────┐                                                                    │
│  │    │  Document: blue | Chunks: green | Entities: amber                  │
│  └────┘                                                                    │
└────────────────────────────────────────────────────────────────────────────┘
```

### 3.4 Custom Graph Nodes

```tsx
// src/components/lineage/graph-nodes/document-node.tsx

import { Handle, Position } from 'reactflow';

interface DocumentNodeData {
  label: string;
  status: DocumentStatus;
  chunkCount: number;
}

export function DocumentGraphNode({ data }: { data: DocumentNodeData }) {
  return (
    <div className="px-4 py-2 rounded-lg bg-blue-100 border-2 border-blue-500 shadow-md">
      <div className="flex items-center gap-2">
        <FileIcon className="h-5 w-5 text-blue-600" />
        <span className="font-medium text-blue-900">{data.label}</span>
      </div>
      <div className="text-xs text-blue-600 mt-1">
        {data.chunkCount} chunks | {data.status}
      </div>
      <Handle type="source" position={Position.Bottom} className="w-2 h-2" />
    </div>
  );
}

// src/components/lineage/graph-nodes/chunk-node.tsx

interface ChunkNodeData {
  index: number;
  preview: string;
  entityCount: number;
  cached: boolean;
}

export function ChunkGraphNode({ data }: { data: ChunkNodeData }) {
  return (
    <div className="px-3 py-2 rounded-lg bg-green-100 border-2 border-green-500 shadow-sm min-w-[100px]">
      <Handle type="target" position={Position.Top} className="w-2 h-2" />
      <div className="flex items-center gap-2">
        <BoxIcon className="h-4 w-4 text-green-600" />
        <span className="font-medium text-green-900">Chunk {data.index + 1}</span>
        {data.cached && <span className="text-xs bg-green-200 px-1 rounded">⚡</span>}
      </div>
      <div className="text-xs text-green-700 truncate max-w-[120px]" title={data.preview}>
        {data.preview}
      </div>
      <div className="text-xs text-green-600 mt-1">
        {data.entityCount} entities
      </div>
      <Handle type="source" position={Position.Bottom} className="w-2 h-2" />
    </div>
  );
}

// src/components/lineage/graph-nodes/entity-node.tsx

interface EntityNodeData {
  name: string;
  entityType: string;
  merged: boolean;
  sourceCount: number;
}

export function EntityGraphNode({ data }: { data: EntityNodeData }) {
  return (
    <div className={cn(
      "px-3 py-2 rounded-lg border-2 shadow-sm",
      data.merged 
        ? "bg-amber-100 border-amber-500" 
        : "bg-yellow-100 border-yellow-500"
    )}>
      <Handle type="target" position={Position.Top} className="w-2 h-2" />
      <div className="flex items-center gap-2">
        <EntityIcon type={data.entityType} className="h-4 w-4" />
        <span className="font-medium text-sm">{data.name}</span>
        {data.merged && <span className="text-xs">⚠️</span>}
      </div>
      <Badge variant="outline" className="mt-1 text-xs">
        {data.entityType}
      </Badge>
      {data.sourceCount > 1 && (
        <div className="text-xs text-muted-foreground mt-1">
          from {data.sourceCount} chunks
        </div>
      )}
    </div>
  );
}
```

### 3.5 Table View Component

```tsx
// src/components/lineage/lineage-table-view.tsx

import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';

export function LineageTableView({
  lineage,
  filter,
  onChunkClick,
  onEntityClick,
}: LineageTableViewProps) {
  const tableData = useMemo(() => {
    if (!lineage) return [];
    
    // Flatten to rows: one per entity with chunk info
    return lineage.entities
      .filter(e => !filter || e.name.toLowerCase().includes(filter.toLowerCase()))
      .flatMap(entity => 
        entity.source_chunks.map(chunkId => {
          const chunk = lineage.chunks.find(c => c.id === chunkId);
          return {
            entityId: entity.id,
            entityName: entity.name,
            entityType: entity.entity_type,
            merged: !!entity.merged_from,
            chunkId: chunkId,
            chunkIndex: chunk?.index ?? 0,
            cached: chunk?.extraction_metadata.cached ?? false,
          };
        })
      );
  }, [lineage, filter]);
  
  const columns: ColumnDef<typeof tableData[0]>[] = [
    {
      accessorKey: 'entityName',
      header: 'Entity',
      cell: ({ row }) => (
        <button
          onClick={() => onEntityClick?.(row.original.entityId)}
          className="flex items-center gap-2 hover:underline"
        >
          <EntityIcon type={row.original.entityType} className="h-4 w-4" />
          {row.original.entityName}
          {row.original.merged && <span className="text-amber-500">⚠️</span>}
        </button>
      ),
    },
    {
      accessorKey: 'entityType',
      header: 'Type',
      cell: ({ getValue }) => <Badge>{getValue() as string}</Badge>,
    },
    {
      accessorKey: 'chunkIndex',
      header: 'Source Chunk',
      cell: ({ row }) => (
        <button
          onClick={() => onChunkClick?.(row.original.chunkId)}
          className="hover:underline"
        >
          Chunk {row.original.chunkIndex + 1}
          {row.original.cached && <span className="ml-1 text-green-500">⚡</span>}
        </button>
      ),
    },
  ];
  
  return (
    <div className="p-4">
      <DataTable columns={columns} data={tableData} />
    </div>
  );
}
```

---

## 4. Interactive Features

### 4.1 Provenance Path Highlighting

```typescript
// On hover, highlight the path from document → chunk → entity

function useProvenanceHighlight(lineage: DocumentLineageResponse | null) {
  const [highlightedPath, setHighlightedPath] = useState<string[]>([]);
  
  const highlightEntity = useCallback((entityId: string) => {
    if (!lineage) return;
    
    const entity = lineage.entities.find(e => e.id === entityId);
    if (!entity) return;
    
    const path = [
      lineage.document.id,
      ...entity.source_chunks,
      entityId,
    ];
    setHighlightedPath(path);
  }, [lineage]);
  
  const clearHighlight = useCallback(() => {
    setHighlightedPath([]);
  }, []);
  
  return { highlightedPath, highlightEntity, clearHighlight };
}
```

### 4.2 Chunk Detail Modal

```tsx
// src/components/lineage/chunk-detail-modal.tsx

interface ChunkDetailModalProps {
  chunk: ChunkLineage | null;
  entities: EntityLineage[];
  open: boolean;
  onClose: () => void;
  onEntityClick: (entityId: string) => void;
}

export function ChunkDetailModal({
  chunk,
  entities,
  open,
  onClose,
  onEntityClick,
}: ChunkDetailModalProps) {
  if (!chunk) return null;
  
  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl max-h-[80vh] overflow-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <BoxIcon className="h-5 w-5" />
            Chunk {chunk.index + 1}
            {chunk.extraction_metadata.cached && (
              <Badge variant="secondary">⚡ Cached</Badge>
            )}
          </DialogTitle>
        </DialogHeader>
        
        {/* Extraction Metadata */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-muted rounded-lg">
          <div>
            <div className="text-xs text-muted-foreground">Model</div>
            <div className="font-mono text-sm">{chunk.extraction_metadata.model}</div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Duration</div>
            <div className="font-mono text-sm">{chunk.extraction_metadata.duration_ms}ms</div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Tokens</div>
            <div className="font-mono text-sm">
              {chunk.extraction_metadata.prompt_tokens} / {chunk.extraction_metadata.completion_tokens}
            </div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Cost</div>
            <div className="font-mono text-sm">${chunk.extraction_metadata.cost_usd.toFixed(4)}</div>
          </div>
        </div>
        
        {/* Chunk Content */}
        <div>
          <h4 className="font-medium mb-2">Content</h4>
          <div className="p-4 bg-muted rounded-lg font-mono text-sm whitespace-pre-wrap max-h-48 overflow-auto">
            {chunk.content_preview}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            Characters {chunk.char_range.start} - {chunk.char_range.end} | {chunk.token_count} tokens
          </div>
        </div>
        
        {/* Extracted Entities */}
        <div>
          <h4 className="font-medium mb-2">Extracted Entities ({entities.length})</h4>
          <div className="space-y-2">
            {entities.map(entity => (
              <button
                key={entity.id}
                onClick={() => onEntityClick(entity.id)}
                className="w-full flex items-center justify-between p-2 hover:bg-muted rounded"
              >
                <div className="flex items-center gap-2">
                  <EntityIcon type={entity.entity_type} />
                  <span>{entity.name}</span>
                  {entity.merged_from && <span className="text-amber-500">⚠️ merged</span>}
                </div>
                <Badge variant="outline">{entity.entity_type}</Badge>
              </button>
            ))}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

### 4.3 Entity Provenance Panel

```tsx
// src/components/lineage/entity-provenance-panel.tsx

interface EntityProvenancePanelProps {
  entityId: string;
  documentId: string;
}

export function EntityProvenancePanel({
  entityId,
  documentId,
}: EntityProvenancePanelProps) {
  const { data: provenance, isLoading } = useEntityProvenance(entityId, documentId);
  
  if (isLoading) return <Skeleton />;
  if (!provenance) return null;
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <EntityIcon type={provenance.entity_type} />
          {provenance.name}
        </CardTitle>
        <CardDescription>{provenance.entity_type}</CardDescription>
      </CardHeader>
      
      <CardContent className="space-y-4">
        {/* Description */}
        <div>
          <h4 className="text-sm font-medium mb-1">Description</h4>
          <p className="text-sm text-muted-foreground">{provenance.description}</p>
        </div>
        
        {/* Merged Entities */}
        {provenance.merged_from && provenance.merged_from.length > 0 && (
          <Alert>
            <AlertTitle className="flex items-center gap-1">
              <MergeIcon className="h-4 w-4" />
              Merged Entity
            </AlertTitle>
            <AlertDescription>
              This entity was merged from: {provenance.merged_from.join(', ')}
            </AlertDescription>
          </Alert>
        )}
        
        {/* Source Chunks */}
        <div>
          <h4 className="text-sm font-medium mb-2">
            Source Chunks ({provenance.source_chunks.length})
          </h4>
          <div className="space-y-2">
            {provenance.source_chunks.map((chunk, index) => (
              <div key={index} className="p-2 bg-muted rounded text-sm">
                <div className="font-medium">Chunk {chunk.chunk_index + 1}</div>
                <div className="text-muted-foreground text-xs mt-1">
                  "{chunk.text_excerpt}"
                </div>
              </div>
            ))}
          </div>
        </div>
        
        {/* Relationships */}
        {provenance.relationships.length > 0 && (
          <div>
            <h4 className="text-sm font-medium mb-2">
              Relationships ({provenance.relationships.length})
            </h4>
            <div className="space-y-1">
              {provenance.relationships.map((rel, index) => (
                <div key={index} className="flex items-center gap-2 text-sm">
                  <span>{provenance.name}</span>
                  <ArrowRightIcon className="h-3 w-3" />
                  <Badge variant="secondary">{rel.relation_type}</Badge>
                  <ArrowRightIcon className="h-3 w-3" />
                  <span>{rel.target_entity_name}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
```

---

## 5. Graph Layout Algorithm

### 5.1 Hierarchical Layout

```typescript
// src/lib/lineage/layout.ts

interface LayoutOptions {
  nodeWidth: number;
  nodeHeight: number;
  horizontalSpacing: number;
  verticalSpacing: number;
}

const defaultOptions: LayoutOptions = {
  nodeWidth: 140,
  nodeHeight: 60,
  horizontalSpacing: 40,
  verticalSpacing: 100,
};

export function calculateHierarchicalLayout(
  lineage: DocumentLineageResponse,
  options: LayoutOptions = defaultOptions
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  
  // Level 0: Document
  const docX = 400;
  nodes.push({
    id: lineage.document.id,
    type: 'document',
    position: { x: docX, y: 0 },
    data: { label: lineage.document.name },
  });
  
  // Level 1: Chunks (evenly distributed)
  const chunkCount = lineage.chunks.length;
  const totalChunkWidth = chunkCount * options.nodeWidth + (chunkCount - 1) * options.horizontalSpacing;
  const chunkStartX = docX - totalChunkWidth / 2 + options.nodeWidth / 2;
  
  lineage.chunks.forEach((chunk, i) => {
    const x = chunkStartX + i * (options.nodeWidth + options.horizontalSpacing);
    const y = options.verticalSpacing;
    
    nodes.push({
      id: chunk.id,
      type: 'chunk',
      position: { x, y },
      data: { index: chunk.index },
    });
    
    edges.push({
      id: `doc-${chunk.id}`,
      source: lineage.document.id,
      target: chunk.id,
    });
  });
  
  // Level 2: Entities (grouped by connected chunk)
  const entityRowY = options.verticalSpacing * 2;
  const chunkEntityMap = new Map<string, EntityLineage[]>();
  
  lineage.entities.forEach(entity => {
    const primaryChunk = entity.source_chunks[0];
    const list = chunkEntityMap.get(primaryChunk) || [];
    list.push(entity);
    chunkEntityMap.set(primaryChunk, list);
  });
  
  chunkEntityMap.forEach((entities, chunkId) => {
    const chunkNode = nodes.find(n => n.id === chunkId);
    if (!chunkNode) return;
    
    const baseX = chunkNode.position.x;
    const totalWidth = entities.length * (options.nodeWidth * 0.8) + (entities.length - 1) * (options.horizontalSpacing * 0.5);
    const startX = baseX - totalWidth / 2 + (options.nodeWidth * 0.8) / 2;
    
    entities.forEach((entity, i) => {
      const x = startX + i * (options.nodeWidth * 0.8 + options.horizontalSpacing * 0.5);
      
      nodes.push({
        id: entity.id,
        type: 'entity',
        position: { x, y: entityRowY },
        data: { name: entity.name, entityType: entity.entity_type },
      });
      
      // Edges from all source chunks
      entity.source_chunks.forEach(srcChunkId => {
        edges.push({
          id: `${srcChunkId}-${entity.id}`,
          source: srcChunkId,
          target: entity.id,
        });
      });
    });
  });
  
  return { nodes, edges };
}
```

---

## 6. Performance Optimization

### 6.1 Virtualization for Large Graphs

```typescript
// Use viewport culling for nodes outside view

import { useViewport } from 'reactflow';

function useVisibleNodes(nodes: Node[], viewport: Viewport) {
  return useMemo(() => {
    const { x, y, zoom } = viewport;
    const visibleWidth = window.innerWidth / zoom;
    const visibleHeight = window.innerHeight / zoom;
    
    return nodes.filter(node => {
      const nodeX = node.position.x;
      const nodeY = node.position.y;
      
      return (
        nodeX >= x - 200 &&
        nodeX <= x + visibleWidth + 200 &&
        nodeY >= y - 200 &&
        nodeY <= y + visibleHeight + 200
      );
    });
  }, [nodes, viewport]);
}
```

### 6.2 Data Pagination

```typescript
// For documents with 1000+ entities, paginate the API response

const { data, hasNextPage, fetchNextPage } = useInfiniteQuery({
  queryKey: ['document-lineage', documentId],
  queryFn: ({ pageParam = 0 }) => 
    getDocumentLineage(documentId, { offset: pageParam, limit: 100 }),
  getNextPageParam: (lastPage, pages) => 
    lastPage.hasMore ? pages.length * 100 : undefined,
});
```

### 6.3 Lazy Loading Entity Details

```typescript
// Only fetch full entity details on click, not on initial load

const [selectedEntityId, setSelectedEntityId] = useState<string | null>(null);

const { data: entityDetails } = useQuery({
  queryKey: ['entity-provenance', selectedEntityId],
  queryFn: () => getEntityProvenance(selectedEntityId!),
  enabled: !!selectedEntityId,
});
```

---

## 7. Accessibility

### 7.1 Keyboard Navigation

```typescript
// Tree view keyboard support

function useTreeKeyboardNav(nodes: TreeNodeData[]) {
  const [focusedIndex, setFocusedIndex] = useState(0);
  
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        setFocusedIndex(prev => Math.min(prev + 1, nodes.length - 1));
        break;
      case 'ArrowUp':
        setFocusedIndex(prev => Math.max(prev - 1, 0));
        break;
      case 'ArrowRight':
        // Expand node
        break;
      case 'ArrowLeft':
        // Collapse node
        break;
      case 'Enter':
      case ' ':
        // Select/activate node
        break;
    }
  }, [nodes]);
  
  return { focusedIndex, handleKeyDown };
}
```

### 7.2 Screen Reader Announcements

```tsx
// Announce graph state changes

<div role="status" aria-live="polite" className="sr-only">
  {selectedNode 
    ? `Selected ${selectedNode.type}: ${selectedNode.data.label}` 
    : 'No node selected'}
</div>

<div role="status" aria-live="polite" className="sr-only">
  Showing {visibleNodes.length} of {totalNodes} nodes. 
  {filter && `Filtered by: ${filter}`}
</div>
```

---

_End of Document WEBUI-006_
