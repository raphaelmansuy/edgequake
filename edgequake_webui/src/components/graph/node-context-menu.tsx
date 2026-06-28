'use client';

import { getEntityTypeColor, formatEntityLabel, formatEntityType } from '@/lib/graph/label-utils';
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
import { useCallback, useEffect, useRef, useState } from 'react';
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
  const menuRef = useRef<HTMLDivElement>(null);

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  // Compute final position:
  // - position.x/y is already the node's right edge (set by graph-renderer using
  //   graphToViewport + nodeScreenRadius + gap).
  // - Shift top up by MENU_H/2 so the menu is vertically centered on the node.
  // - Clamp to viewport bounds so the menu never clips off screen.
  const safePos = useCallback(() => {
    if (!position) return { left: 0, top: 0 };
    const W = window.innerWidth;
    const H = window.innerHeight;
    const MENU_W = 224;
    const MENU_H = 320; // approximate — 6 items × ~40px + header ~60px
    const centeredTop = position.y - MENU_H / 2;
    return {
      left: Math.min(Math.max(position.x, 8), W - MENU_W - 8),
      top:  Math.min(Math.max(centeredTop,  8), H - MENU_H - 8),
    };
  }, [position]);

  if (!node || !position) return null;

  const displayLabel = formatEntityLabel(node.label ?? '', 40);
  const displayType  = formatEntityType(node.node_type ?? '');
  const typeColor    = getEntityTypeColor(node.node_type);
  const pos = safePos();

  // Reusable menu item builder — keeps JSX DRY
  const Item = ({
    icon: Icon,
    label,
    kbd,
    danger = false,
    check,
    onClick,
  }: {
    icon: React.FC<{ className?: string }>;
    label: string;
    kbd?: string;
    danger?: boolean;
    check?: boolean;
    onClick: () => void;
  }) => (
    <button
      className={[
        'flex items-center gap-2 w-full px-2 py-1.5 text-sm rounded-sm transition-colors',
        danger
          ? 'text-destructive hover:bg-destructive/10 hover:text-destructive'
          : 'hover:bg-accent hover:text-accent-foreground',
      ].join(' ')}
      onClick={() => { onClick(); handleClose(); }}
    >
      <Icon className="h-4 w-4 shrink-0" />
      <span className="flex-1 text-left">{label}</span>
      {check && <span className="text-xs text-muted-foreground">✓</span>}
      {kbd && (
        <kbd className="ml-auto text-[10px] font-mono bg-muted/70 text-muted-foreground px-1.5 py-0.5 rounded border border-border/50">
          {kbd}
        </kbd>
      )}
    </button>
  );

  return (
    <div
      ref={menuRef}
      className="fixed z-50"
      style={{ left: pos.left, top: pos.top }}
    >
      <div className="bg-popover border rounded-lg shadow-lg p-1 min-w-56">
        {/* Header: formatted name + type with color dot */}
        <div className="px-2.5 py-2 border-b mb-1">
          <div
            className="font-semibold text-sm truncate"
            title={formatEntityLabel(node.label ?? '', 200)}
          >
            {displayLabel}
          </div>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span
              className="inline-block w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: typeColor }}
              aria-hidden="true"
            />
            <span className="text-xs text-muted-foreground">{displayType}</span>
          </div>
        </div>

        <Item icon={Eye}     label={t('graph.contextMenu.viewDetails', 'View Details')}          kbd="↵"  onClick={() => onViewDetails(node)} />
        <Item icon={Network} label={t('graph.contextMenu.expandNeighborhood', 'Expand Neighborhood')} check={isExpanded} onClick={() => onExpandNeighborhood(node)} />
        {onPruneNode && (
          <Item icon={Minimize2} label={t('graph.contextMenu.pruneNode', 'Prune Node')} onClick={() => onPruneNode!(node)} />
        )}
        <Item icon={Search}  label={t('graph.contextMenu.findRelated', 'Find Related')}                       onClick={() => onFindRelated(node)} />

        <div className="my-1 h-px bg-border" />

        <Item icon={FileText} label={t('graph.contextMenu.viewDocuments', 'View Documents')}           onClick={() => onViewDocuments(node)} />
        <Item icon={Copy}     label={t('graph.contextMenu.copyId', 'Copy Entity ID')}        kbd="⌘C" onClick={() => onCopyId(node)} />

        {onDelete && (
          <>
            <div className="my-1 h-px bg-border" />
            <Item icon={Trash2} label={t('graph.contextMenu.deleteEntity', 'Delete Entity')} danger onClick={() => onDelete!(node)} />
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
