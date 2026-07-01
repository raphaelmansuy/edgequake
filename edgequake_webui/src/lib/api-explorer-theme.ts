/**
 * @module api-explorer-theme
 * @description Maps EdgeQuake design tokens to Scalar CSS variables.
 *
 * Uses CSS custom properties from globals.css so light/dark follow the app theme
 * automatically (no duplicate palette maintenance).
 *
 * Embedded-mode layout overrides: Scalar defaults to 100dvh standalone sizing;
 * inside the EdgeQuake dashboard we constrain height and enable internal scroll.
 *
 * @see https://scalar.com/products/api-references/integrations/react
 * @enforces DRY - token mapping in one module
 * @enforces SRP - styling only
 */

/** Map EdgeQuake shadcn tokens → Scalar variables (works in light and dark). */
export const SCALAR_TOKEN_BRIDGE_CSS = `
[data-testid="api-explorer-scalar"] .scalar-app {
  --scalar-background-1: var(--background);
  --scalar-background-2: var(--card);
  --scalar-background-3: var(--muted);
  --scalar-background-accent: color-mix(in oklch, var(--primary) 12%, transparent);

  --scalar-color-1: var(--foreground);
  --scalar-color-2: var(--muted-foreground);
  --scalar-color-3: var(--muted-foreground);

  --scalar-color-accent: var(--primary);
  --scalar-border-color: var(--border);
  --scalar-border-radius: var(--radius);

  --scalar-color-green:  hsl(142 71% 45%);
  --scalar-color-red:    hsl(0 84% 60%);
  --scalar-color-yellow: hsl(47.9 95.8% 53.1%);
  --scalar-color-blue:   hsl(217.2 91.2% 59.8%);
  --scalar-color-orange: hsl(24.6 95% 53.1%);
  --scalar-color-purple: hsl(262.1 83.3% 57.8%);

  --scalar-sidebar-background-1: var(--sidebar);
  --scalar-sidebar-color-1: var(--sidebar-foreground);
  --scalar-sidebar-color-2: var(--muted-foreground);
  --scalar-sidebar-color-active: var(--sidebar-primary);
  --scalar-sidebar-border-color: var(--sidebar-border);

  --scalar-code-background: var(--muted);
  --scalar-code-color: var(--foreground);
}
`;

/**
 * Layout + UX overrides for dashboard embedding.
 * Never target `.references-sidebar` with max-width — classic/modern roots can carry
 * that class and the entire pane collapses to ~352px (SPEC-035 visual QC regression).
 */
export const SCALAR_LAYOUT_AND_CHROME_CSS = `
[data-testid="api-explorer-scalar"] {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  background: var(--background);
  color: var(--foreground);
}

/* Scalar React wrapper divs default to content height — constrain to dashboard pane */
[data-testid="api-explorer-scalar"] > div,
[data-testid="api-explorer-scalar"] > div > div {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  height: 100%;
  max-height: 100%;
  overflow: hidden;
}

/* Override Scalar standalone 100dvh sizing — fill dashboard main pane */
[data-testid="api-explorer-scalar"] .scalar-api-reference,
[data-testid="api-explorer-scalar"] .scalar-app,
[data-testid="api-explorer-scalar"] .references-layout {
  width: 100% !important;
  max-width: 100% !important;
  height: 100% !important;
  min-height: 0 !important;
  max-height: 100% !important;
  overflow: hidden !important;
  --full-height: 100%;
}

[data-testid="api-explorer-scalar"] .narrow-references-container {
  width: 100%;
  max-width: 100%;
  height: 100%;
  min-height: 0;
  overflow-x: hidden !important;
  overflow-y: auto !important;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
}

/* Modern layout: content flows inside narrow-references-container (scroll parent) */
[data-testid="api-explorer-scalar"] .references-rendered {
  height: auto !important;
  max-height: none !important;
  min-height: 0 !important;
  overflow: visible !important;
}

[data-testid="api-explorer-scalar"] .t-doc__sidebar {
  height: 100% !important;
  max-height: 100% !important;
  min-height: 0 !important;
  overflow-x: hidden !important;
  overflow-y: auto !important;
  overscroll-behavior: contain;
}

[data-testid="api-explorer-scalar"] .scalar-container {
  height: 100%;
  min-height: 0;
}

/* Hide Scalar developer toolbar (Configure / Share / Deploy) */
[data-testid="api-explorer-scalar"] .references-toolbar,
[data-testid="api-explorer-scalar"] [class*="references-toolbar"],
[data-testid="api-explorer-scalar"] .t-doc__toolbar {
  display: none !important;
}

/* Hide Ask AI — not part of EdgeQuake product (FEAT-035) */
[data-testid="api-explorer-scalar"] [class*="ask-ai"],
[data-testid="api-explorer-scalar"] [data-testid="ask-ai"],
[data-testid="api-explorer-scalar"] button[aria-label*="Ask AI" i],
[data-testid="api-explorer-scalar"] a[aria-label*="Ask AI" i] {
  display: none !important;
}

/* Hide Scalar footer promo */
[data-testid="api-explorer-scalar"] .scalar-footer,
[data-testid="api-explorer-scalar"] [class*="powered-by"] {
  display: none !important;
}
`;

/** Auth + intro layout polish — tenant/workspace header schemes, stacked intro cards. */
export const SCALAR_AUTH_AND_INTRO_CSS = `
/* Embedded dashboard: avoid side-by-side intro columns squeezing auth fields */
[data-testid="api-explorer-scalar"] .section-columns {
  flex-direction: column !important;
  gap: 24px !important;
}

[data-testid="api-explorer-scalar"] .section-column {
  width: 100% !important;
  max-width: 100% !important;
}

[data-testid="api-explorer-scalar"] .introduction-card,
[data-testid="api-explorer-scalar"] .introduction-card-row {
  flex-direction: column !important;
  align-items: stretch !important;
  gap: 12px !important;
}

[data-testid="api-explorer-scalar"] .introduction-card-row > *,
[data-testid="api-explorer-scalar"] .introduction-card-item {
  width: 100% !important;
  max-width: 100% !important;
  min-width: 0 !important;
  flex: 1 1 auto !important;
}

[data-testid="api-explorer-scalar"] .scalar-reference-intro-auth,
[data-testid="api-explorer-scalar"] .scalar-reference-intro-server,
[data-testid="api-explorer-scalar"] .scalar-reference-intro-clients {
  width: 100%;
}

/* Auth tables: single-column rows, consistent cell padding (intro + try-it-out) */
[data-testid="api-explorer-scalar"] .scalar-data-table .grid.auto-rows-auto,
[data-testid="api-explorer-scalar"] .scalar-data-table .grid.min-h-8 {
  grid-template-columns: 1fr !important;
  row-gap: 0.25rem;
}

[data-testid="api-explorer-scalar"] .scalar-data-table td {
  padding-block: 0.25rem;
  vertical-align: middle;
}

[data-testid="api-explorer-scalar"] .scalar-data-table td .relative.flex {
  width: 100%;
}

/*
 * Header names are fixed in OpenAPI — hide redundant Name rows (intro auth table order:
 * bearer label, bearer token, tenant label, tenant name, tenant value, workspace label, workspace name, workspace value)
 */
[data-testid="api-explorer-scalar"] .scalar-data-table tr.group.contents:nth-child(4),
[data-testid="api-explorer-scalar"] .scalar-data-table tr.group.contents:nth-child(7) {
  display: none !important;
}

/* Try-it-out auth: compact scheme section headers */
[data-testid="api-explorer-scalar"] .scalar-data-table tr.group.contents td[colspan] {
  font-weight: 600;
  padding-top: 0.5rem;
}
`;

/** Combined CSS passed to Scalar customCss. */
export const SCALAR_CUSTOM_CSS =
  SCALAR_TOKEN_BRIDGE_CSS +
  SCALAR_LAYOUT_AND_CHROME_CSS +
  SCALAR_AUTH_AND_INTRO_CSS;
