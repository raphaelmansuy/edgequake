'use client';

import { useGraphStore } from '@/stores/use-graph-store';
import { useTheme } from 'next-themes';
import { useCallback, useEffect, useRef, useState } from 'react';
import type Sigma from 'sigma';

interface MinimapProps {
  /** Width of the minimap */
  width?: number;
  /** Height of the minimap */
  height?: number;
  /** Position of the minimap (optional, for self-positioning) */
  position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
}

/**
 * Graph Minimap Component
 * 
 * Displays a scaled-down overview of the graph with a viewport rectangle
 * showing the currently visible area. Allows click-to-navigate.
 */
export function GraphMinimap({ 
  width = 120, 
  height = 100,
  position
}: MinimapProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const [isDragging, setIsDragging] = useState(false);
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === 'dark';

  // Colors based on theme
  const bgColor = isDark ? 'rgba(30, 30, 30, 0.9)' : 'rgba(255, 255, 255, 0.95)';
  const borderColor = isDark ? 'rgba(100, 100, 100, 0.6)' : 'rgba(200, 200, 200, 0.8)';
  const nodeColor = isDark ? 'rgba(255, 255, 255, 0.6)' : 'rgba(0, 0, 0, 0.5)';
  const viewportColor = isDark ? 'rgba(59, 130, 246, 0.4)' : 'rgba(59, 130, 246, 0.3)';
  const viewportBorderColor = isDark ? 'rgba(59, 130, 246, 0.8)' : 'rgba(59, 130, 246, 0.7)';

  // Get graph bounding box
  const getGraphBBox = useCallback((sigma: Sigma) => {
    const graph = sigma.getGraph();
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    
    graph.forEachNode((nodeId) => {
      const x = graph.getNodeAttribute(nodeId, 'x');
      const y = graph.getNodeAttribute(nodeId, 'y');
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      if (y < minY) minY = y;
      if (y > maxY) maxY = y;
    });

    // Add padding
    const padX = (maxX - minX) * 0.1 || 10;
    const padY = (maxY - minY) * 0.1 || 10;
    
    return {
      x: minX - padX,
      y: minY - padY,
      width: (maxX - minX) + padX * 2 || 20,
      height: (maxY - minY) + padY * 2 || 20,
    };
  }, []);

  // Draw the minimap
  const drawMinimap = useCallback((sigma: Sigma) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const graph = sigma.getGraph();
    const bbox = getGraphBBox(sigma);

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw background
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    // Draw border
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.strokeRect(0.5, 0.5, width - 1, height - 1);

    // Calculate scale to fit graph in minimap
    const scaleX = (width - 4) / bbox.width;
    const scaleY = (height - 4) / bbox.height;
    const scale = Math.min(scaleX, scaleY);

    // Center offset
    const offsetX = (width - bbox.width * scale) / 2;
    const offsetY = (height - bbox.height * scale) / 2;

    // Transform function
    const transform = (x: number, y: number) => ({
      x: (x - bbox.x) * scale + offsetX,
      y: (y - bbox.y) * scale + offsetY,
    });

    // Draw edges (simplified as lines)
    ctx.strokeStyle = isDark ? 'rgba(100, 100, 100, 0.3)' : 'rgba(150, 150, 150, 0.3)';
    ctx.lineWidth = 0.5;
    graph.forEachEdge((edge, attrs, source, target) => {
      const sourceX = graph.getNodeAttribute(source, 'x');
      const sourceY = graph.getNodeAttribute(source, 'y');
      const targetX = graph.getNodeAttribute(target, 'x');
      const targetY = graph.getNodeAttribute(target, 'y');
      
      const p1 = transform(sourceX, sourceY);
      const p2 = transform(targetX, targetY);
      
      ctx.beginPath();
      ctx.moveTo(p1.x, p1.y);
      ctx.lineTo(p2.x, p2.y);
      ctx.stroke();
    });

    // Draw nodes
    graph.forEachNode((nodeId) => {
      const x = graph.getNodeAttribute(nodeId, 'x');
      const y = graph.getNodeAttribute(nodeId, 'y');
      const color = graph.getNodeAttribute(nodeId, 'color') || nodeColor;
      const size = Math.max(1.5, (graph.getNodeAttribute(nodeId, 'size') || 5) * scale * 0.3);
      
      const pos = transform(x, y);
      
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, size, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
    });

    // Calculate and draw viewport rectangle
    const camera = sigma.getCamera();
    const cameraState = camera.getState();
    
    // Get visible area in graph coordinates
    const container = sigma.getContainer();
    const viewWidth = container.clientWidth / cameraState.ratio;
    const viewHeight = container.clientHeight / cameraState.ratio;
    
    // Viewport position in graph coords
    const viewX = cameraState.x - viewWidth / 2;
    const viewY = cameraState.y - viewHeight / 2;

    // Transform to minimap coords
    const vpTopLeft = transform(viewX, viewY);
    const vpBottomRight = transform(viewX + viewWidth, viewY + viewHeight);
    
    const vpRect = {
      x: vpTopLeft.x,
      y: vpTopLeft.y,
      width: vpBottomRight.x - vpTopLeft.x,
      height: vpBottomRight.y - vpTopLeft.y,
    };

    // Clamp viewport to minimap bounds
    vpRect.x = Math.max(0, Math.min(vpRect.x, width - vpRect.width));
    vpRect.y = Math.max(0, Math.min(vpRect.y, height - vpRect.height));
    vpRect.width = Math.max(10, Math.min(vpRect.width, width));
    vpRect.height = Math.max(10, Math.min(vpRect.height, height));

    // Draw viewport rectangle
    ctx.fillStyle = viewportColor;
    ctx.fillRect(vpRect.x, vpRect.y, vpRect.width, vpRect.height);
    ctx.strokeStyle = viewportBorderColor;
    ctx.lineWidth = 1.5;
    ctx.strokeRect(vpRect.x, vpRect.y, vpRect.width, vpRect.height);

  }, [width, height, bgColor, borderColor, nodeColor, viewportColor, viewportBorderColor, isDark, getGraphBBox]);

  // Handle click to navigate
  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!sigmaInstance || !canvasRef.current) return;

    const canvas = canvasRef.current;
    const rect = canvas.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    const bbox = getGraphBBox(sigmaInstance);
    
    // Calculate scale
    const scaleX = (width - 4) / bbox.width;
    const scaleY = (height - 4) / bbox.height;
    const scale = Math.min(scaleX, scaleY);
    
    // Center offset
    const offsetX = (width - bbox.width * scale) / 2;
    const offsetY = (height - bbox.height * scale) / 2;

    // Inverse transform: minimap coords to graph coords
    const graphX = (clickX - offsetX) / scale + bbox.x;
    const graphY = (clickY - offsetY) / scale + bbox.y;

    // Animate camera to this position
    sigmaInstance.getCamera().animate(
      { x: graphX, y: graphY },
      { duration: 300 }
    );
  }, [sigmaInstance, width, height, getGraphBBox]);

  // Handle drag navigation
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    setIsDragging(true);
    handleClick(e);
  }, [handleClick]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!isDragging) return;
    handleClick(e);
  }, [isDragging, handleClick]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Subscribe to camera updates
  useEffect(() => {
    if (!sigmaInstance) return;

    const redraw = () => drawMinimap(sigmaInstance);

    // Initial draw
    redraw();

    // Listen to camera updates
    sigmaInstance.getCamera().on('updated', redraw);

    // Also redraw on graph updates
    const graph = sigmaInstance.getGraph();
    const handlers = {
      nodeAdded: redraw,
      nodeDropped: redraw,
      nodeAttributesUpdated: redraw,
      edgeAdded: redraw,
      edgeDropped: redraw,
    };

    Object.entries(handlers).forEach(([event, handler]) => {
      graph.on(event as keyof typeof handlers, handler);
    });

    // Cleanup
    return () => {
      sigmaInstance.getCamera().off('updated', redraw);
      Object.entries(handlers).forEach(([event, handler]) => {
        graph.off(event as keyof typeof handlers, handler);
      });
    };
  }, [sigmaInstance, drawMinimap]);

  // Position classes (only applied if position prop is specified)
  const positionClasses = {
    'top-left': 'absolute top-4 left-4',
    'top-right': 'absolute top-4 right-4',
    'bottom-left': 'absolute bottom-4 left-4',
    'bottom-right': 'absolute bottom-4 right-4',
  };

  if (!sigmaInstance) return null;

  return (
    <div 
      className={`z-10 ${position ? positionClasses[position] : ''}`}
      style={{ 
        width, 
        height,
        boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
        borderRadius: '6px',
        overflow: 'hidden',
      }}
    >
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        style={{ 
          cursor: isDragging ? 'grabbing' : 'crosshair',
          display: 'block',
        }}
        onClick={handleClick}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        title="Click to navigate"
      />
    </div>
  );
}

export default GraphMinimap;
