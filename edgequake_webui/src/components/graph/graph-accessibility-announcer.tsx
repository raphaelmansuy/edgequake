'use client';

/**
 * @module GraphAccessibilityAnnouncer
 * @description Screen reader announcer for graph node selection.
 * Uses aria-live region to announce node selection changes.
 *
 * @implements FEAT0641 - Screen reader support for graph
 * @implements UC0307 - User navigates graph with screen reader
 *
 * @enforces BR0627 - Accessible graph navigation
 * 
 * @see WCAG 2.1.1 Keyboard (Level A)
 * @see WCAG 4.1.2 Name, Role, Value (Level A)
 */

import { useGraphStore } from '@/stores/use-graph-store';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * WHY: Screen readers need aria-live regions to announce dynamic content changes.
 * This component watches selectedNodeId and announces changes to assistive tech.
 * Uses role="status" with aria-live="polite" so it doesn't interrupt current reading.
 */
export function GraphAccessibilityAnnouncer() {
  const { t } = useTranslation();
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  
  const [announcement, setAnnouncement] = useState('');
  
  // WHY: Update announcement when selected node changes
  // This triggers screen reader to read the new content
  useEffect(() => {
    if (!selectedNodeId) {
      setAnnouncement(t('graph.a11y.noSelection', 'No node selected'));
      return;
    }
    
    const node = nodes.find(n => n.id === selectedNodeId);
    if (!node) {
      setAnnouncement(t('graph.a11y.nodeNotFound', 'Node not found'));
      return;
    }
    
    // Calculate degree (number of connections)
    const degree = edges.filter(
      e => e.source === selectedNodeId || e.target === selectedNodeId
    ).length;
    
    // Build announcement message
    const label = node.label || node.id;
    const type = node.node_type || t('graph.a11y.unknownType', 'unknown type');
    const connectionsText = degree === 1 
      ? t('graph.a11y.oneConnection', '1 connection')
      : t('graph.a11y.connections', '{{count}} connections', { count: degree });
    
    setAnnouncement(
      t('graph.a11y.selectedNode', 'Selected: {{label}}, type {{type}}, {{connections}}', {
        label,
        type,
        connections: connectionsText,
      })
    );
  }, [selectedNodeId, nodes, edges, t]);
  
  // WHY: sr-only class hides the element visually but keeps it accessible
  // role="status" + aria-live="polite" ensures screen readers announce changes
  // aria-atomic="true" announces the entire content, not just changes
  return (
    <div 
      role="status" 
      aria-live="polite" 
      aria-atomic="true"
      className="sr-only"
    >
      {announcement}
    </div>
  );
}

export default GraphAccessibilityAnnouncer;
