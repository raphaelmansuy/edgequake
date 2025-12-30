'use client';

import type { GraphNode } from '@/types';
import {
    Copy,
    Eye,
    FileText,
    Minimize2,
    Network,
    Search,
    Trash2
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface NodeContextMenuPosition {
  x: number;
  y: number;
}

interface NodeContextMenuProps {
  node: GraphNode | null;
  position: NodeContextMenuPosition | null;
  onClose: () => void;
  onViewDetails: (node: GraphNode) => void;
  onExpandNeighborhood: (node: GraphNode) => void;
  onPruneNode?: (node: GraphNode) => void;
  onFindRelated: (node: GraphNode) => void;
  onViewDocuments: (node: GraphNode) => void;
  onCopyId: (node: GraphNode) => void;
  onDelete?: (node: GraphNode) => void;
  isExpanded?: boolean;
}

export function NodeContextMenu({
  node,
  position,
  onClose,
  onViewDetails,
  onExpandNeighborhood,
  onPruneNode,
  onFindRelated,
  onViewDocuments,
  onCopyId,
  onDelete,
  isExpanded = false,
}: NodeContextMenuProps) {
  const { t } = useTranslation();

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  if (!node || !position) return null;

  return (
    <div
      className="fixed z-50"
      style={{ left: position.x, top: position.y }}
    >
      <div className="bg-popover border rounded-md shadow-md p-1 min-w-[200px]">
        <div className="px-2 py-1.5 border-b mb-1">
          <div className="font-medium truncate max-w-[180px]">{node.label}</div>
          <div className="text-xs text-muted-foreground">{node.node_type}</div>
        </div>

        <button
          className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
          onClick={() => {
            onViewDetails(node);
            handleClose();
          }}
        >
          <Eye className="h-4 w-4" />
          <span>{t('graph.contextMenu.viewDetails', 'View Details')}</span>
          <span className="ml-auto text-xs text-muted-foreground">Enter</span>
        </button>

        <button
          className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
          onClick={() => {
            onExpandNeighborhood(node);
            handleClose();
          }}
        >
          <Network className="h-4 w-4" />
          <span>{t('graph.contextMenu.expandNeighborhood', 'Expand Neighborhood')}</span>
          {isExpanded && (
            <span className="ml-auto text-xs text-muted-foreground">✓</span>
          )}
        </button>

        {onPruneNode && (
          <button
            className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
            onClick={() => {
              onPruneNode(node);
              handleClose();
            }}
          >
            <Minimize2 className="h-4 w-4" />
            <span>{t('graph.contextMenu.pruneNode', 'Prune Node')}</span>
          </button>
        )}

        <button
          className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
          onClick={() => {
            onFindRelated(node);
            handleClose();
          }}
        >
          <Search className="h-4 w-4" />
          <span>{t('graph.contextMenu.findRelated', 'Find Related Entities')}</span>
        </button>

        <div className="my-1 h-px bg-border" />

        <button
          className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
          onClick={() => {
            onViewDocuments(node);
            handleClose();
          }}
        >
          <FileText className="h-4 w-4" />
          <span>{t('graph.contextMenu.viewDocuments', 'View Source Documents')}</span>
        </button>

        <button
          className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground transition-colors"
          onClick={() => {
            onCopyId(node);
            handleClose();
          }}
        >
          <Copy className="h-4 w-4" />
          <span>{t('graph.contextMenu.copyId', 'Copy Entity ID')}</span>
          <span className="ml-auto text-xs text-muted-foreground">⌘C</span>
        </button>

        {onDelete && (
          <>
            <div className="my-1 h-px bg-border" />
            <button
              className="flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm hover:bg-destructive hover:text-destructive-foreground transition-colors text-destructive"
              onClick={() => {
                onDelete(node);
                handleClose();
              }}
            >
              <Trash2 className="h-4 w-4" />
              <span>{t('graph.contextMenu.deleteEntity', 'Delete Entity')}</span>
            </button>
          </>
        )}
      </div>
    </div>
  );
}

// Hook to manage context menu state
export function useNodeContextMenu() {
  const [contextMenuState, setContextMenuState] = useState<{
    node: GraphNode | null;
    position: { x: number; y: number } | null;
  }>({ node: null, position: null });

  const openContextMenu = useCallback((node: GraphNode, x: number, y: number) => {
    setContextMenuState({ node, position: { x, y } });
  }, []);

  const closeContextMenu = useCallback(() => {
    setContextMenuState({ node: null, position: null });
  }, []);

  // Close on escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && contextMenuState.node) {
        closeContextMenu();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [contextMenuState.node, closeContextMenu]);

  // Close on click outside
  useEffect(() => {
    const handleClick = () => {
      if (contextMenuState.node) {
        closeContextMenu();
      }
    };

    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, [contextMenuState.node, closeContextMenu]);

  return {
    contextMenuNode: contextMenuState.node,
    contextMenuPosition: contextMenuState.position,
    openContextMenu,
    closeContextMenu,
  };
}

export default NodeContextMenu;
