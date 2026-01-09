"use client";

/**
 * @module use-graph-keyboard-navigation
 * @description Custom hook for keyboard navigation in the graph viewer.
 * Provides accessibility features for graph interaction.
 *
 * @implements UC0306 - User navigates graph with keyboard
 * @implements FEAT0639 - Keyboard navigation for graph nodes
 * @implements FEAT0640 - Focus management for accessibility
 *
 * @enforces BR0625 - Escape key resets selection
 * @enforces BR0626 - Arrow keys navigate between nodes
 */

import {
  focusCameraOnNode,
  resetCameraToFitGraph,
} from "@/lib/graph/camera-utils";
import { useGraphStore } from "@/stores/use-graph-store";
import { useCallback, useEffect } from "react";

export interface GraphKeyboardOptions {
  /** Enable/disable keyboard navigation */
  enabled?: boolean;
  /** Callback when a node is focused via keyboard */
  onNodeFocus?: (nodeId: string) => void;
  /** Callback when graph is deselected via Escape */
  onDeselect?: () => void;
}

/**
 * Custom hook for keyboard navigation in the graph viewer.
 *
 * Keyboard shortcuts:
 * - Arrow keys: Navigate between nodes
 * - Enter: Focus camera on selected node
 * - Escape: Deselect current node
 * - +/=: Zoom in
 * - -/_: Zoom out
 * - 0: Reset zoom to fit all
 * - F: Toggle fullscreen
 * - Tab: Cycle to next node
 * - Shift+Tab: Cycle to previous node
 */
export function useGraphKeyboardNavigation(options: GraphKeyboardOptions = {}) {
  const { enabled = true, onNodeFocus, onDeselect } = options;

  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const nodes = useGraphStore((s) => s.nodes);
  const selectNode = useGraphStore((s) => s.selectNode);
  const clearSelection = useGraphStore((s) => s.clearSelection);

  // Get sorted node IDs for consistent navigation
  const sortedNodeIds = nodes
    .sort((a, b) => (a.label || a.id).localeCompare(b.label || b.id))
    .map((n) => n.id);

  // Navigate to next/previous node
  const navigateToNode = useCallback(
    (direction: "next" | "prev") => {
      if (sortedNodeIds.length === 0) return;

      let nextIndex: number;

      if (!selectedNodeId) {
        // No selection, start from beginning or end
        nextIndex = direction === "next" ? 0 : sortedNodeIds.length - 1;
      } else {
        const currentIndex = sortedNodeIds.indexOf(selectedNodeId);
        if (currentIndex === -1) {
          nextIndex = 0;
        } else {
          nextIndex =
            direction === "next"
              ? (currentIndex + 1) % sortedNodeIds.length
              : (currentIndex - 1 + sortedNodeIds.length) %
                sortedNodeIds.length;
        }
      }

      const nextNodeId = sortedNodeIds[nextIndex];
      selectNode(nextNodeId);
      onNodeFocus?.(nextNodeId);

      // Optionally focus camera on the new node
      if (sigmaInstance) {
        focusCameraOnNode(sigmaInstance, nextNodeId, {
          ratio: 0.5,
          duration: 300,
          highlight: false,
        });
      }
    },
    [sortedNodeIds, selectedNodeId, selectNode, sigmaInstance, onNodeFocus]
  );

  // Zoom controls
  const handleZoomIn = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedZoom({ duration: 200, factor: 1.5 });
    }
  }, [sigmaInstance]);

  const handleZoomOut = useCallback(() => {
    if (sigmaInstance) {
      const camera = sigmaInstance.getCamera();
      camera.animatedUnzoom({ duration: 200, factor: 1.5 });
    }
  }, [sigmaInstance]);

  const handleResetZoom = useCallback(() => {
    if (sigmaInstance) {
      resetCameraToFitGraph(sigmaInstance, 500);
    }
  }, [sigmaInstance]);

  const handleFocusSelected = useCallback(() => {
    if (sigmaInstance && selectedNodeId) {
      focusCameraOnNode(sigmaInstance, selectedNodeId, {
        ratio: 0.4,
        duration: 500,
        highlight: true,
      });
    }
  }, [sigmaInstance, selectedNodeId]);

  const handleDeselect = useCallback(() => {
    clearSelection();
    onDeselect?.();
  }, [clearSelection, onDeselect]);

  const handleFullscreen = useCallback(() => {
    const container = document.querySelector("[data-graph-container]");
    if (!container) return;

    if (!document.fullscreenElement) {
      const isDark = document.documentElement.classList.contains("dark");
      if (isDark) {
        container.classList.add("dark");
      }
      container.requestFullscreen?.();
    } else {
      container.classList.remove("dark");
      document.exitFullscreen?.();
    }
  }, []);

  // Main keyboard handler
  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Don't interfere with input fields
      const target = event.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      // Handle various keyboard shortcuts
      switch (event.key) {
        case "Tab":
          // Tab cycles through nodes
          event.preventDefault();
          navigateToNode(event.shiftKey ? "prev" : "next");
          break;

        case "ArrowRight":
        case "ArrowDown":
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            navigateToNode("next");
          }
          break;

        case "ArrowLeft":
        case "ArrowUp":
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            navigateToNode("prev");
          }
          break;

        case "Enter":
          // Focus camera on selected node
          if (selectedNodeId) {
            event.preventDefault();
            handleFocusSelected();
          }
          break;

        case "Escape":
          // Deselect current node
          event.preventDefault();
          handleDeselect();
          break;

        case "+":
        case "=":
          // Zoom in
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleZoomIn();
          }
          break;

        case "-":
        case "_":
          // Zoom out
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleZoomOut();
          }
          break;

        case "0":
          // Reset zoom
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleResetZoom();
          }
          break;

        case "f":
        case "F":
          // Toggle fullscreen
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleFullscreen();
          }
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    enabled,
    navigateToNode,
    selectedNodeId,
    handleFocusSelected,
    handleDeselect,
    handleZoomIn,
    handleZoomOut,
    handleResetZoom,
    handleFullscreen,
  ]);

  return {
    navigateToNode,
    handleZoomIn,
    handleZoomOut,
    handleResetZoom,
    handleFocusSelected,
    handleDeselect,
    handleFullscreen,
  };
}

export default useGraphKeyboardNavigation;
