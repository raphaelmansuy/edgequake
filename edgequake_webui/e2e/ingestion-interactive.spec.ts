/**
 * Interactive E2E Tests for Ingestion Pipeline
 *
 * Tests document upload, real-time progress tracking, and lineage visualization.
 * Run with: pnpm exec playwright test ingestion-interactive.spec.ts --headed
 *
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 */

import { expect, test } from "@playwright/test";
import fs from "fs";
import path from "path";

// Screenshot output directory
const SCREENSHOT_DIR = "e2e/screenshots/ingestion-interactive";

// Test document content
const TEST_DOCUMENT_CONTENT = `
# EdgeQuake Research Document

## Abstract

EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework 
designed for knowledge graph construction and semantic search.

## Key Technologies

The system leverages several cutting-edge technologies:

1. **Apache AGE** - A PostgreSQL extension for graph database capabilities
2. **pgvector** - Vector similarity search for embeddings
3. **OpenAI GPT-4** - Large language model for entity extraction

## Architecture Components

### Core Services

The EdgeQuake architecture consists of:

- **API Layer**: RESTful endpoints built with Axum web framework
- **Pipeline Service**: Document ingestion and entity extraction
- **Storage Layer**: Multi-tenant graph and vector storage

### Entity Extraction

The pipeline extracts entities such as:
- SARAH_CHEN: Lead researcher in AI systems
- DR_MARTINEZ: Expert in knowledge graphs
- PROJECT_ALPHA: Main research initiative

Sarah Chen collaborates with Dr. Martinez on Project Alpha.
Dr. Martinez leads the knowledge graph development team.
Project Alpha aims to integrate RAG with graph databases.

## Conclusions

EdgeQuake represents a significant advancement in RAG technology,
combining graph databases with vector search for enhanced retrieval.
`;

test.describe("Interactive Ingestion Pipeline Tests", () => {
  test.beforeAll(async () => {
    // Ensure screenshot directory exists
    if (!fs.existsSync(SCREENSHOT_DIR)) {
      fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
    }
  });

  test.beforeEach(async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");
  });

  test("01 - Navigate to documents page and view upload zone", async ({
    page,
  }) => {
    // Take initial screenshot
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "01-documents-page.png"),
      fullPage: true,
    });

    // Verify page loads
    const hasContent =
      (await page.getByText(/documents|upload|drag/i).count()) > 0;
    expect(hasContent).toBeTruthy();
  });

  test("02 - Upload document and track progress @interactive", async ({
    page,
  }) => {
    // Create a temporary test file
    const testFilePath = path.join(process.cwd(), "test-upload-temp.md");
    fs.writeFileSync(testFilePath, TEST_DOCUMENT_CONTENT);

    try {
      // Look for file input or dropzone
      const fileInput = page.locator('input[type="file"]');

      // If hidden, we need to handle the dropzone
      if ((await fileInput.count()) > 0) {
        // Direct file input
        await fileInput.setInputFiles(testFilePath);
      } else {
        // Try the dropzone approach
        const dropzone = page.locator(
          '[data-testid="dropzone"], .dropzone, [role="button"]:has-text("Drag")'
        );
        if ((await dropzone.count()) > 0) {
          // Use page.setInputFiles with a selector
          await page.setInputFiles('input[type="file"]', testFilePath);
        }
      }

      // Wait a moment for upload to start
      await page.waitForTimeout(1000);

      // Take screenshot after upload initiated
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "02-upload-initiated.png"),
        fullPage: true,
      });

      // Check for progress indicators
      const hasProgress =
        (await page
          .locator('[role="progressbar"], .progress, [data-testid="progress"]')
          .count()) > 0 ||
        (await page
          .getByText(/uploading|processing|extracting|progress/i)
          .count()) > 0;

      // Take screenshot of progress (if visible)
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "02-progress-tracking.png"),
        fullPage: true,
      });

      // Wait for potential processing to complete (with timeout)
      await page.waitForTimeout(5000);

      // Final screenshot
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "02-upload-complete.png"),
        fullPage: true,
      });
    } finally {
      // Clean up test file
      if (fs.existsSync(testFilePath)) {
        fs.unlinkSync(testFilePath);
      }
    }
  });

  test("03 - View document list with processing status", async ({ page }) => {
    // Wait for documents to load
    await page.waitForTimeout(1000);

    // Check for documents table
    const hasTable = (await page.locator('table, [role="table"]').count()) > 0;
    const hasDocuments = (await page.locator('tr, [role="row"]').count()) > 1;

    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "03-document-list.png"),
      fullPage: true,
    });

    // Check for status badges
    const hasStatusBadges =
      (await page.getByText(/completed|processing|pending|failed/i).count()) >
      0;

    // Log findings
    console.log(
      `Table present: ${hasTable}, Documents found: ${hasDocuments}, Status badges: ${hasStatusBadges}`
    );
  });

  test("04 - View WebSocket connection status", async ({ page }) => {
    // Look for WebSocket status indicator
    const wsStatus = page.locator(
      '[data-testid="websocket-status"], .ws-status, [aria-label*="connection"]'
    );

    // Take screenshot of connection status
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "04-websocket-status.png"),
      fullPage: true,
    });

    // Listen for WebSocket events
    let wsConnected = false;
    page.on("websocket", (ws) => {
      console.log(`WebSocket opened: ${ws.url()}`);
      wsConnected = true;
    });

    // Wait a bit for potential WS connection
    await page.waitForTimeout(2000);
  });

  test("05 - Navigate to costs page", async ({ page }) => {
    // Find and click costs link
    const costsLink = page.getByRole("link", { name: /costs/i });

    if (await costsLink.isVisible()) {
      await costsLink.click();
      await page.waitForLoadState("networkidle");

      // Verify we're on costs page
      await expect(page).toHaveURL(/\/costs/);

      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "05-costs-page.png"),
        fullPage: true,
      });

      // Check for cost-related content
      const hasCostContent =
        (await page.getByText(/cost|budget|usage|tokens/i).count()) > 0;

      console.log(`Cost page content found: ${hasCostContent}`);
    } else {
      console.log("Costs link not visible, skipping navigation");
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "05-no-costs-link.png"),
        fullPage: true,
      });
    }
  });

  test("06 - View graph visualization", async ({ page }) => {
    // Navigate to graph page
    await page.goto("/graph");
    await page.waitForLoadState("networkidle");

    await page.waitForTimeout(1000);

    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "06-graph-page.png"),
      fullPage: true,
    });

    // Check for graph elements
    const hasGraph =
      (await page.locator('canvas, svg, [data-testid="graph"]').count()) > 0;
    const hasEmptyState =
      (await page
        .getByText(/no.*graph|empty|upload.*document|no entities/i)
        .count()) > 0;

    console.log(`Graph present: ${hasGraph}, Empty state: ${hasEmptyState}`);
  });

  test("07 - View document detail with lineage @interactive", async ({
    page,
  }) => {
    // Navigate to documents page first
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Find and click on first document view link
    const viewLinks = page.getByRole("link", { name: /view/i });

    if ((await viewLinks.count()) > 0) {
      await viewLinks.first().click();
      await page.waitForLoadState("networkidle");

      await page.waitForTimeout(500);

      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "07-document-detail.png"),
        fullPage: true,
      });

      // Check for lineage section
      const hasLineage =
        (await page
          .getByText(/extraction lineage|lineage|pipeline|chunks|entities/i)
          .count()) > 0;

      console.log(`Lineage section found: ${hasLineage}`);

      // Look for expandable tree or graph
      const hasTreeOrGraph =
        (await page
          .locator(
            '[data-testid="lineage-tree"], [data-testid="lineage-graph"]'
          )
          .count()) > 0;

      if (hasTreeOrGraph) {
        await page.screenshot({
          path: path.join(SCREENSHOT_DIR, "07-lineage-detail.png"),
          fullPage: true,
        });
      }
    } else {
      console.log("No documents to view");
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "07-no-documents.png"),
        fullPage: true,
      });
    }
  });

  test("08 - Check ingestion progress panel @interactive", async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Check for any active ingestion progress panels
    const progressPanel = page.locator(
      '[data-testid="ingestion-progress"], .ingestion-progress'
    );

    if ((await progressPanel.count()) > 0) {
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "08-progress-panel.png"),
        fullPage: true,
      });

      // Check for stage indicators
      const hasStages =
        (await page
          .getByText(
            /preprocessing|chunking|extracting|merging|embedding|indexing/i
          )
          .count()) > 0;

      console.log(`Stage indicators found: ${hasStages}`);
    } else {
      console.log("No active ingestion progress panels");
      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, "08-no-active-progress.png"),
        fullPage: true,
      });
    }
  });
});

test.describe("Stage Indicator Visualization Tests", () => {
  test("09 - Verify stage indicator styles", async ({ page }) => {
    // Navigate to documents and look for stage indicators
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Take screenshot of any visible stage indicators
    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "09-stage-indicators.png"),
      fullPage: true,
    });

    // Check for expected stage names
    const stages = [
      "preprocessing",
      "chunking",
      "extracting",
      "merging",
      "embedding",
      "indexing",
    ];

    for (const stage of stages) {
      const hasStage =
        (await page.getByText(new RegExp(stage, "i")).count()) > 0;
      if (hasStage) {
        console.log(`Stage "${stage}" found in UI`);
      }
    }
  });
});

test.describe("Cost Tracking UI Tests", () => {
  test("10 - Verify cost badge displays", async ({ page }) => {
    // Navigate to documents page
    await page.goto("/documents");
    await page.waitForLoadState("networkidle");

    // Look for cost badges
    const costBadges = page.locator('[data-testid="cost-badge"], .cost-badge');
    const costText = await page.getByText(/\$[0-9.]+/).count();

    await page.screenshot({
      path: path.join(SCREENSHOT_DIR, "10-cost-badges.png"),
      fullPage: true,
    });

    console.log(
      `Cost badges found: ${await costBadges.count()}, Cost text matches: ${costText}`
    );
  });
});
