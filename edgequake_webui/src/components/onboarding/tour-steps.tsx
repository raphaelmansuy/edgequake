/**
 * @fileoverview Tour step definitions for graph viewer onboarding
 *
 * @implements FEAT1052 - Graph viewer tour steps
 * @implements FEAT1053 - Contextual help content
 *
 * @see UC1303 - User learns graph navigation
 * @see UC1304 - User discovers keyboard shortcuts
 *
 * @enforces BR1052 - Accessible tour content
 * @enforces BR1053 - Progressive disclosure of features
 */
'use client';

import type { TourStep } from './tour-provider';

/**
 * Tour steps for the Graph Viewer page.
 * These guide users through the main features of the knowledge graph.
 */
export const GRAPH_TOUR_STEPS: TourStep[] = [
  {
    id: 'welcome',
    target: '[data-tour="graph-header"]',
    title: 'Welcome to the Knowledge Graph',
    content: (
      <>
        <p>This is your knowledge graph visualization. Here you can explore entities, relationships, and insights extracted from your documents.</p>
        <p className="mt-2 text-xs">Let&apos;s take a quick tour of the main features!</p>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'entity-browser',
    target: '[data-tour="entity-browser"]',
    title: 'Entity Browser',
    content: (
      <>
        <p>Browse all entities in your knowledge graph. You can search, filter by type, and sort by name or connection count.</p>
        <p className="mt-2 text-xs">Click on any entity to select it and see its details.</p>
      </>
    ),
    placement: 'right',
  },
  {
    id: 'graph-canvas',
    target: '[data-tour="graph-canvas"]',
    title: 'Interactive Graph',
    content: (
      <>
        <p>The main visualization shows entities as nodes and relationships as edges.</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• Click a node to select it</li>
          <li>• Drag nodes to rearrange</li>
          <li>• Scroll to zoom in/out</li>
          <li>• Right-click for context menu</li>
        </ul>
      </>
    ),
    placement: 'left',
  },
  {
    id: 'search',
    target: '[data-tour="graph-search"]',
    title: 'Quick Search',
    content: (
      <>
        <p>Find any entity instantly. Start typing to search by name, type, or description.</p>
        <p className="mt-2 text-xs">Tip: Press <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">⌘K</kbd> for quick access!</p>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'layout',
    target: '[data-tour="layout-control"]',
    title: 'Layout Options',
    content: (
      <>
        <p>Change how the graph is arranged:</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• <strong>Force</strong> - Physics-based clustering</li>
          <li>• <strong>Circular</strong> - Nodes in a circle</li>
          <li>• <strong>Random</strong> - Fresh arrangement</li>
        </ul>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'zoom-controls',
    target: '[data-tour="zoom-controls"]',
    title: 'Zoom & Camera',
    content: (
      <>
        <p>Control your view of the graph:</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• Zoom in/out for detail</li>
          <li>• Rotate the view</li>
          <li>• Reset to fit all nodes</li>
          <li>• Toggle fullscreen</li>
        </ul>
      </>
    ),
    placement: 'left',
  },
  {
    id: 'details-panel',
    target: '[data-tour="details-panel"]',
    title: 'Details & Filters',
    content: (
      <>
        <p>When you select a node, see its full details here including:</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• Entity type and description</li>
          <li>• Connected relationships</li>
          <li>• Source documents</li>
        </ul>
        <p className="mt-2 text-xs">You can also filter which entity types are visible.</p>
      </>
    ),
    placement: 'left',
  },
  {
    id: 'keyboard',
    target: '[data-tour="keyboard-help"]',
    title: 'Keyboard Shortcuts',
    content: (
      <>
        <p>Power users love keyboard shortcuts! Click this button to see all available shortcuts.</p>
        <p className="mt-2 text-xs">Try using <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Tab</kbd> to navigate between nodes!</p>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'complete',
    target: '[data-tour="graph-header"]',
    title: 'You\'re Ready! 🎉',
    content: (
      <>
        <p>That&apos;s the basics! Explore your knowledge graph and discover insights.</p>
        <p className="mt-2 text-xs">You can restart this tour anytime from the help menu.</p>
      </>
    ),
    placement: 'bottom',
  },
];

/**
 * Tour steps for the Documents page.
 */
export const DOCUMENTS_TOUR_STEPS: TourStep[] = [
  {
    id: 'welcome',
    target: '[data-tour="documents-header"]',
    title: 'Document Management',
    content: (
      <>
        <p>Upload and manage your documents here. Each document is processed to extract entities and relationships for the knowledge graph.</p>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'upload',
    target: '[data-tour="upload-button"]',
    title: 'Upload Documents',
    content: (
      <>
        <p>Click here to upload new documents. Supported formats include:</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• Plain text (.txt)</li>
          <li>• Markdown (.md)</li>
          <li>• PDF documents</li>
        </ul>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'list',
    target: '[data-tour="document-list"]',
    title: 'Document List',
    content: (
      <>
        <p>View all your uploaded documents. Click on a document to see its details and the entities extracted from it.</p>
      </>
    ),
    placement: 'top',
  },
];

/**
 * Tour steps for the Query page.
 */
export const QUERY_TOUR_STEPS: TourStep[] = [
  {
    id: 'welcome',
    target: '[data-tour="query-header"]',
    title: 'Query Your Knowledge',
    content: (
      <>
        <p>Ask questions about your documents using natural language. The system will search the knowledge graph and generate informed answers.</p>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'input',
    target: '[data-tour="query-input"]',
    title: 'Ask a Question',
    content: (
      <>
        <p>Type your question here. Be specific for better results!</p>
        <p className="mt-2 text-xs">Examples:</p>
        <ul className="text-xs space-y-1">
          <li>• &quot;What are the main topics discussed?&quot;</li>
          <li>• &quot;Who is mentioned in the documents?&quot;</li>
          <li>• &quot;What relationships exist between X and Y?&quot;</li>
        </ul>
      </>
    ),
    placement: 'bottom',
  },
  {
    id: 'mode',
    target: '[data-tour="query-mode"]',
    title: 'Query Modes',
    content: (
      <>
        <p>Choose how to query your knowledge:</p>
        <ul className="mt-2 text-xs space-y-1">
          <li>• <strong>Local</strong> - Fast, focused answers</li>
          <li>• <strong>Global</strong> - Comprehensive search</li>
          <li>• <strong>Hybrid</strong> - Best of both</li>
        </ul>
      </>
    ),
    placement: 'bottom',
  },
];

export default GRAPH_TOUR_STEPS;
