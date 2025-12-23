'use client';

import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuLabel,
    ContextMenuSeparator,
} from '@/components/ui/context-menu';
import type { GraphNode } from '@/types';
import {
    Copy,
    Expand,
    Eye,
    FileText,
    Link2,
    Search,
    Trash2,
} from 'lucide-react';
import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export interface NodeContextMenuAction {
  type:
    | 'view-details'
    | 'expand-neighborhood'
    | 'find-related'
    | 'view-documents'
    | 'copy-id'
    | 'focus-node'
    | 'hide-node';
  node: GraphNode;
  position?: { x: number; y: number };
}

interface GraphContextMenuProps {
  children: React.ReactNode;
  node: GraphNode | null;
  onAction?: (action: NodeContextMenuAction) => void;
}

/**
 * Context menu for graph nodes with various actions
 */
export const GraphContextMenu = memo(function GraphContextMenu({
  children,
  node,
  onAction,
}: GraphContextMenuProps) {
  const { t } = useTranslation();

  const handleCopyId = () => {
    if (node) {
      navigator.clipboard.writeText(node.id);
      toast.success(t('common.copied', 'Copied!'));
    }
  };

  const handleAction = (type: NodeContextMenuAction['type']) => {
    if (node && onAction) {
      onAction({ type, node });
    }
  };

  if (!node) {
    return <>{children}</>;
  }

  return (
    <ContextMenu>
      {children}
      <ContextMenuContent className="w-56">
        <ContextMenuLabel className="flex items-center gap-2 font-normal">
          <span className="font-semibold truncate">{node.label}</span>
          <span className="text-xs text-muted-foreground">({node.node_type})</span>
        </ContextMenuLabel>
        <ContextMenuSeparator />
        
        <ContextMenuItem onClick={() => handleAction('view-details')}>
          <Eye className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.viewDetails', 'View Details')}
        </ContextMenuItem>
        
        <ContextMenuItem onClick={() => handleAction('expand-neighborhood')}>
          <Expand className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.expandNeighborhood', 'Expand Neighborhood')}
        </ContextMenuItem>
        
        <ContextMenuItem onClick={() => handleAction('find-related')}>
          <Search className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.findRelated', 'Find Related')}
        </ContextMenuItem>

        <ContextMenuSeparator />
        
        <ContextMenuItem onClick={() => handleAction('view-documents')}>
          <FileText className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.viewDocuments', 'View Documents')}
        </ContextMenuItem>
        
        <ContextMenuItem onClick={() => handleAction('focus-node')}>
          <Link2 className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.focusNode', 'Focus on Node')}
        </ContextMenuItem>

        <ContextMenuSeparator />
        
        <ContextMenuItem onClick={handleCopyId}>
          <Copy className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.copyId', 'Copy ID')}
        </ContextMenuItem>
        
        <ContextMenuItem
          onClick={() => handleAction('hide-node')}
          className="text-destructive focus:text-destructive"
        >
          <Trash2 className="h-4 w-4 mr-2" />
          {t('graph.contextMenu.hideNode', 'Hide Node')}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
});

/**
 * Standalone context menu content for use with custom positioning
 */
export interface GraphNodeMenuState {
  visible: boolean;
  x: number;
  y: number;
  node: GraphNode | null;
}

interface StandaloneGraphContextMenuProps {
  state: GraphNodeMenuState;
  onClose: () => void;
  onAction?: (action: NodeContextMenuAction) => void;
}

export function StandaloneGraphContextMenu({
  state,
  onClose,
  onAction,
}: StandaloneGraphContextMenuProps) {
  const { t } = useTranslation();
  const { visible, x, y, node } = state;

  if (!visible || !node) return null;

  const handleCopyId = () => {
    navigator.clipboard.writeText(node.id);
    toast.success(t('common.copied', 'Copied!'));
    onClose();
  };

  const handleAction = (type: NodeContextMenuAction['type']) => {
    if (onAction) {
      onAction({ type, node, position: { x, y } });
    }
    onClose();
  };

  return (
    <>
      {/* Backdrop to close menu */}
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      {/* Menu */}
      <div
        className="fixed z-50 min-w-[200px] rounded-md border bg-popover p-1 text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95"
        style={{ left: x, top: y }}
      >
        {/* Node label header */}
        <div className="px-2 py-1.5 text-sm">
          <span className="font-semibold truncate">{node.label}</span>
          <span className="text-xs text-muted-foreground ml-1">({node.node_type})</span>
        </div>
        <div className="h-px bg-border my-1" />

        {/* Menu items */}
        <MenuItem
          icon={Eye}
          label={t('graph.contextMenu.viewDetails', 'View Details')}
          onClick={() => handleAction('view-details')}
        />
        <MenuItem
          icon={Expand}
          label={t('graph.contextMenu.expandNeighborhood', 'Expand Neighborhood')}
          onClick={() => handleAction('expand-neighborhood')}
        />
        <MenuItem
          icon={Search}
          label={t('graph.contextMenu.findRelated', 'Find Related')}
          onClick={() => handleAction('find-related')}
        />

        <div className="h-px bg-border my-1" />

        <MenuItem
          icon={FileText}
          label={t('graph.contextMenu.viewDocuments', 'View Documents')}
          onClick={() => handleAction('view-documents')}
        />
        <MenuItem
          icon={Link2}
          label={t('graph.contextMenu.focusNode', 'Focus on Node')}
          onClick={() => handleAction('focus-node')}
        />

        <div className="h-px bg-border my-1" />

        <MenuItem
          icon={Copy}
          label={t('graph.contextMenu.copyId', 'Copy ID')}
          onClick={handleCopyId}
        />
        <MenuItem
          icon={Trash2}
          label={t('graph.contextMenu.hideNode', 'Hide Node')}
          onClick={() => handleAction('hide-node')}
          destructive
        />
      </div>
    </>
  );
}

function MenuItem({
  icon: Icon,
  label,
  onClick,
  destructive,
}: {
  icon: typeof Eye;
  label: string;
  onClick: () => void;
  destructive?: boolean;
}) {
  return (
    <button
      className={`relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground ${
        destructive ? 'text-destructive hover:text-destructive' : ''
      }`}
      onClick={onClick}
    >
      <Icon className="h-4 w-4 mr-2" />
      {label}
    </button>
  );
}

export default GraphContextMenu;
