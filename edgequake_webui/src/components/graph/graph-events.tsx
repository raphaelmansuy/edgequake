'use client';

import { useRegisterEvents, useSigma } from '@react-sigma/core';
import { useEffect, useState } from 'react';

/**
 * GraphEvents component that enables node drag & drop functionality.
 * This component should be placed inside SigmaContainer.
 */
export function GraphEvents() {
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const [draggedNode, setDraggedNode] = useState<string | null>(null);

  useEffect(() => {
    // Register sigma event handlers
    registerEvents({
      // Mouse down on node starts drag
      downNode: (e) => {
        setDraggedNode(e.node);
        sigma.getGraph().setNodeAttribute(e.node, 'highlighted', true);
      },

      // Mouse move updates node position when dragging
      mousemovebody: (e) => {
        if (!draggedNode) return;

        // Get position in graph coordinates
        const pos = sigma.viewportToGraph(e);
        const graph = sigma.getGraph();

        // Update node position
        graph.setNodeAttribute(draggedNode, 'x', pos.x);
        graph.setNodeAttribute(draggedNode, 'y', pos.y);

        // Prevent sigma default behavior (camera movement)
        e.preventSigmaDefault();
        e.original.preventDefault();
        e.original.stopPropagation();
      },

      // Mouse up ends drag
      mouseup: () => {
        if (draggedNode) {
          sigma.getGraph().removeNodeAttribute(draggedNode, 'highlighted');
          setDraggedNode(null);
        }
      },

      // Prevent camera auto-positioning while dragging
      mousedown: (e) => {
        const mouseEvent = e.original as MouseEvent;
        // If mouse button is pressed and we don't have a custom bounding box yet
        if (mouseEvent.buttons !== 0 && !sigma.getCustomBBox()) {
          sigma.setCustomBBox(sigma.getBBox());
        }
      },
    });
  }, [registerEvents, sigma, draggedNode]);

  return null;
}

export default GraphEvents;
