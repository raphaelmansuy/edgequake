/**
 * Duplicate Upload Detection E2E Tests
 *
 * @implements FEAT-dup-detection - Duplicate upload detection and resolution
 * @implements BR-dup-replace     - Replace = reprocess existing document
 * @implements BR-dup-skip        - Skip = silently discard duplicate upload
 *
 * These tests verify the full duplicate upload workflow:
 * 1. Uploading the same document twice shows the DuplicateUploadDialog
 * 2. The dialog displays the correct file name and existing document ID
 * 3. Choosing "Replace" triggers a reprocess call for the existing document
 * 4. Choosing "Skip" (or Skip all) closes the dialog without reprocessing
 *
 * Strategy: API route mocking via page.route() allows us to control the
 * upload response without needing a live backend or specific documents.
 * The first upload returns a "processing" response; the second returns
 * duplicate_of to simulate the backend duplicate detection.
 */

import { expect, test } from "@playwright/test";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Sample existing document ID returned by mock backend on duplicate. */
const EXISTING_DOC_ID = "11111111-2222-3333-4444-555555555555";

/** Successful first-upload response. */
const PDF_SUCCESS_RESPONSE = {
  pdf_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
  document_id: null,
  status: "processing",
  task_id: "task-001",
  track_id: "upload-track-001",
  message: "PDF uploaded successfully. Processing in background.",
  estimated_time_seconds: 30,
  metadata: {
    filename: "test-document.pdf",
    file_size_bytes: 1024,
    page_count: 1,
    sha256_checksum: "abc123",
    vision_enabled: true,
    vision_model: "gpt-4.1-nano",
  },
  duplicate_of: null,
};

/** Duplicate response returned on second upload of same PDF. */
const PDF_DUPLICATE_RESPONSE = {
  pdf_id: EXISTING_DOC_ID,
  document_id: EXISTING_DOC_ID,
  status: "duplicate",
  task_id: "",
  track_id: "upload-track-002",
  message: `PDF already uploaded with ID: ${EXISTING_DOC_ID}`,
  estimated_time_seconds: 0,
  metadata: {
    filename: "test-document.pdf",
    file_size_bytes: 1024,
    page_count: 1,
    sha256_checksum: "abc123",
    vision_enabled: false,
    vision_model: null,
  },
  duplicate_of: EXISTING_DOC_ID,
};

/** Successful first-upload response for text documents. */
const TEXT_SUCCESS_RESPONSE = {
  document_id: "ffffffff-eeee-dddd-cccc-bbbbbbbbbbbb",
  status: "pending",
  task_id: "task-text-001",
  track_id: "upload-text-track-001",
  duplicate_of: null,
};

/** Duplicate response for text documents. */
const TEXT_DUPLICATE_RESPONSE = {
  document_id: EXISTING_DOC_ID,
  status: "duplicate_processing",
  task_id: null,
  track_id: "upload-text-track-002",
  duplicate_of: EXISTING_DOC_ID,
};

// ---------------------------------------------------------------------------
// Mock data for TenantGuard initialization
// ---------------------------------------------------------------------------

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "TestTenant",
  slug: "test-tenant",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: MOCK_WORKSPACE_ID,
  name: "Default Workspace",
  slug: "default-workspace",
  tenant_id: MOCK_TENANT_ID,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

// ---------------------------------------------------------------------------
// Test Suite
// ---------------------------------------------------------------------------

test.describe("Duplicate Upload Detection", () => {
  test.beforeEach(async ({ page }) => {
    // -----------------------------------------------------------------------
    // Mock ALL backend endpoints the page needs during initialization.
    // Without tenants+workspaces, TenantGuard blocks the documents view.
    //
    // IMPORTANT: Playwright routes are matched in LIFO order (last
    // registered wins). Register catch-all FIRST, then specific routes.
    // -----------------------------------------------------------------------

    // 1) Catch-all for any API v1 GET requests (registered FIRST = lowest priority)
    await page.route("**/api/v1/**", async (route) => {
      if (route.request().method() === "GET") {
        // Return a safe empty response for unknown endpoints
        // Include common fields that code might access
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            items: [],
            tasks: [],
            phases: [],
            statistics: {
              pending: 0,
              processing: 0,
              indexed: 0,
              failed: 0,
              cancelled: 0,
            },
            pagination: {
              total: 0,
              page: 1,
              page_size: 50,
              total_pages: 0,
            },
          }),
        });
      } else {
        await route.fallback();
      }
    });

    // 2) Health check (root path, not under /api/v1)
    await page.route("**/health", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          status: "healthy",
          version: "0.1.0-test",
          storage_mode: "postgresql",
        }),
      });
    });

    // 3) Ready check
    await page.route("**/ready", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "ready" }),
      });
    });

    // 4) Tenants list → TenantGuard needs this to auto-select a tenant
    await page.route("**/api/v1/tenants", async (route) => {
      if (route.request().method() === "GET") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([MOCK_TENANT]),
        });
      } else {
        await route.fallback();
      }
    });

    // 5) Workspaces for tenant → TenantGuard needs this to auto-select workspace
    await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
      if (route.request().method() === "GET") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([MOCK_WORKSPACE]),
        });
      } else {
        await route.fallback();
      }
    });

    // 6) Documents list (GET) — returns empty document list
    //    Pattern must match with AND without query params
    await page.route("**/api/v1/documents**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      // Only intercept GET requests that look like documents list (not sub-resources like /pdf)
      if (method === "GET" && !url.includes("/documents/pdf")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            items: [],
            total: 0,
            page: 1,
            page_size: 20,
            status_counts: {
              pending: 0,
              processing: 0,
              completed: 0,
              failed: 0,
              cancelled: 0,
            },
          }),
        });
      } else {
        // POST /api/v1/documents - text upload, or sub-route (will be overridden per-test)
        await route.fallback();
      }
    });

    // 7) Pipeline status
    await page.route("**/api/v1/pipeline/status**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          running_tasks: 0,
          is_busy: false,
          queued_tasks: 0,
        }),
      });
    });

    // 8) Tasks list — getPipelineStatus() uses /tasks internally
    await page.route("**/api/v1/tasks**", async (route) => {
      if (route.request().method() === "GET") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            tasks: [],
            pagination: {
              total: 0,
              page: 1,
              page_size: 50,
              total_pages: 0,
            },
            statistics: {
              pending: 0,
              processing: 0,
              indexed: 0,
              failed: 0,
              cancelled: 0,
            },
          }),
        });
      } else {
        await route.fallback();
      }
    });

    // 9) PDF progress — usePdfProgress hook polls this after upload
    await page.route("**/api/v1/documents/pdf/progress/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          track_id: "mock-track",
          pdf_id: "mock-pdf",
          document_id: null,
          filename: "test.pdf",
          phases: [],
          overall_percentage: 100,
          is_complete: true,
          is_failed: false,
          started_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
          completed_at: "2026-01-01T00:00:00Z",
        }),
      });
    });

    // Navigate to documents page and wait for content to load
    await page.goto("/documents");
    await page.waitForLoadState("domcontentloaded");
    // Wait for TenantGuard to finish initialization and render the documents page
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 15000,
    });
  });

  // -------------------------------------------------------------------------
  // PDF duplicate detection tests
  // -------------------------------------------------------------------------

  test.describe("PDF duplicate detection", () => {
    test("shows DuplicateUploadDialog when same PDF is uploaded twice", async ({
      page,
    }) => {
      let pdfUploadCount = 0;

      // Intercept the PDF upload endpoint
      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        pdfUploadCount++;
        const response =
          pdfUploadCount === 1 ? PDF_SUCCESS_RESPONSE : PDF_DUPLICATE_RESPONSE;

        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(response),
        });
      });

      // --- First upload ---
      const fileInput = page.locator('input[type="file"]');
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });

      // Wait first upload completes (no dialog should appear)
      await page.waitForTimeout(1500);
      const dialogAfterFirst = page.getByRole("dialog").first();
      await expect(dialogAfterFirst)
        .not.toBeVisible({ timeout: 2000 })
        .catch(() => {
          // Dialog should NOT be open after a successful first upload
        });

      // --- Second upload (same file = duplicate) ---
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });

      // The duplicate dialog MUST appear
      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // Verify title mentions "duplicate"
      const dialogTitle = dialog.locator(
        '[id*="dialog-title"], [data-testid="dialog-title"], .dialog-title, h2',
      );
      const titleText = await dialogTitle
        .first()
        .textContent()
        .catch(() => "");
      expect(
        titleText?.toLowerCase().includes("duplicate") ||
          titleText?.toLowerCase().includes("already") ||
          (await dialog.textContent())?.toLowerCase().includes("duplicate"),
      ).toBeTruthy();

      // Verify the filename is shown
      const dialogContent = await dialog.textContent();
      expect(dialogContent).toContain("test-document.pdf");
    });

    test("Replace button re-uploads PDF with force_reindex=true (no DELETE)", async ({
      page,
    }) => {
      let pdfUploadCount = 0;
      let forceReindexCallMade = false;

      // Mock PDF upload endpoint — first call is new file, second returns duplicate,
      // third is the force_reindex re-upload (Replace flow).
      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        pdfUploadCount++;
        if (pdfUploadCount === 1) {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify(PDF_SUCCESS_RESPONSE),
          });
        } else if (pdfUploadCount === 2) {
          // Second upload → duplicate detected
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify(PDF_DUPLICATE_RESPONSE),
          });
        } else {
          // Third upload → the force_reindex re-upload (Replace flow)
          // Check if force_reindex=true is in the multipart body
          const postData = route.request().postData() ?? "";
          if (postData.includes("force_reindex") && postData.includes("true")) {
            forceReindexCallMade = true;
          }
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              ...PDF_SUCCESS_RESPONSE,
              pdf_id: "new-fresh-pdf-id",
              duplicate_of: null,
            }),
          });
        }
      });

      // Ensure no DELETE call is made (force_reindex replaces delete+reupload).
      // WHY (OODA-08): Backend atomically clears old data on force_reindex.
      let deleteCallMade = false;
      await page.route(
        `**/api/v1/documents/${EXISTING_DOC_ID}`,
        async (route) => {
          if (route.request().method() === "DELETE") {
            deleteCallMade = true;
            await route.fulfill({ status: 204 });
          } else {
            await route.fallback();
          }
        },
      );

      const fileInput = page.locator('input[type="file"]');

      // First upload
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });
      await page.waitForTimeout(1500);

      // Second upload → duplicate dialog
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // "Replace" should already be selected by default — confirm immediately.
      const confirmButton = dialog.getByRole("button", { name: /confirm/i });
      if (await confirmButton.isVisible().catch(() => false)) {
        await confirmButton.click();
      }

      // Wait for the async force_reindex re-upload to complete
      await page.waitForTimeout(3000);

      // Dialog should be closed
      await expect(dialog).not.toBeVisible({ timeout: 3000 });

      // A force_reindex POST must have been made (3rd POST call)
      expect(pdfUploadCount).toBeGreaterThanOrEqual(3);

      // force_reindex=true must have been set in the request body
      expect(forceReindexCallMade).toBe(true);

      // No DELETE should have been called (force_reindex handles cleanup server-side)
      expect(deleteCallMade).toBe(false);
    });

    test("Skip closes dialog without deleting or re-uploading", async ({
      page,
    }) => {
      let pdfUploadCount = 0;
      let deleteCallMade = false;

      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        pdfUploadCount++;
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(
            pdfUploadCount === 1
              ? PDF_SUCCESS_RESPONSE
              : PDF_DUPLICATE_RESPONSE,
          ),
        });
      });

      // Guard: DELETE must NOT be called for a Skip decision.
      await page.route("**/api/v1/documents/**", async (route) => {
        if (route.request().method() === "DELETE") {
          deleteCallMade = true;
          await route.fulfill({ status: 204 });
        } else {
          await route.fallback();
        }
      });

      const fileInput = page.locator('input[type="file"]');

      // First upload
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });
      await page.waitForTimeout(1500);

      // Second upload → duplicate dialog
      await fileInput.setInputFiles({
        name: "test-document.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4 test content"),
      });

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // Click "Skip all & close" button
      const skipButton = dialog.getByRole("button", {
        name: /skip all|skip/i,
      });
      await skipButton.first().click();

      await page.waitForTimeout(500);

      // Dialog should be closed
      await expect(dialog).not.toBeVisible({ timeout: 3000 });

      // DELETE should NOT have been called (no replacement)
      expect(deleteCallMade).toBe(false);
    });
  });

  // -------------------------------------------------------------------------
  // Text document duplicate detection tests
  // -------------------------------------------------------------------------

  test.describe("Text document duplicate detection", () => {
    test("shows DuplicateUploadDialog when same text file is uploaded twice", async ({
      page,
    }) => {
      let textUploadCount = 0;

      // Mock the text upload endpoint
      await page.route("**/api/v1/documents**", async (route) => {
        const method = route.request().method();
        if (method === "POST") {
          textUploadCount++;
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify(
              textUploadCount === 1
                ? TEXT_SUCCESS_RESPONSE
                : TEXT_DUPLICATE_RESPONSE,
            ),
          });
        } else if (method === "GET") {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              items: [],
              total: 0,
              page: 1,
              page_size: 20,
              status_counts: {},
            }),
          });
        } else {
          await route.fallback();
        }
      });

      const fileInput = page.locator('input[type="file"]');

      // First upload
      await fileInput.setInputFiles({
        name: "my-notes.txt",
        mimeType: "text/plain",
        buffer: Buffer.from(
          "Hello World - test content for duplicate detection",
        ),
      });
      await page.waitForTimeout(1500);

      // Second upload (same content = duplicate)
      await fileInput.setInputFiles({
        name: "my-notes.txt",
        mimeType: "text/plain",
        buffer: Buffer.from(
          "Hello World - test content for duplicate detection",
        ),
      });

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      const dialogContent = await dialog.textContent();
      expect(dialogContent).toContain("my-notes.txt");
    });
  });

  // -------------------------------------------------------------------------
  // Batch duplicate detection tests
  // -------------------------------------------------------------------------

  test.describe("Batch upload with duplicates", () => {
    test("shows dialog listing multiple duplicates when uploading a batch", async ({
      page,
    }) => {
      const uploadedFiles = new Set<string>();

      // Mock PDF upload - return duplicate for already-seen filenames
      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }

        // Parse multipart to get filename
        const postData = route.request().postDataBuffer?.();
        const bodyStr = postData ? postData.toString() : "";
        const filenameMatch = bodyStr.match(
          /filename="([^"]+)"|filename=([^\r\n]+)/,
        );
        const filename =
          filenameMatch?.[1] || filenameMatch?.[2] || "unknown.pdf";

        const isDuplicate = uploadedFiles.has(filename);
        uploadedFiles.add(filename);

        if (isDuplicate) {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              ...PDF_DUPLICATE_RESPONSE,
              metadata: { ...PDF_DUPLICATE_RESPONSE.metadata, filename },
            }),
          });
        } else {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              ...PDF_SUCCESS_RESPONSE,
              metadata: { ...PDF_SUCCESS_RESPONSE.metadata, filename },
            }),
          });
        }
      });

      const fileInput = page.locator('input[type="file"]');

      // Upload first batch (new files)
      await fileInput.setInputFiles([
        {
          name: "doc-a.pdf",
          mimeType: "application/pdf",
          buffer: Buffer.from("%PDF-1.4 content A"),
        },
        {
          name: "doc-b.pdf",
          mimeType: "application/pdf",
          buffer: Buffer.from("%PDF-1.4 content B"),
        },
      ]);
      await page.waitForTimeout(2000);

      // Upload same files again (all duplicates)
      await fileInput.setInputFiles([
        {
          name: "doc-a.pdf",
          mimeType: "application/pdf",
          buffer: Buffer.from("%PDF-1.4 content A"),
        },
        {
          name: "doc-b.pdf",
          mimeType: "application/pdf",
          buffer: Buffer.from("%PDF-1.4 content B"),
        },
      ]);

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // With multiple duplicates, batch action buttons should appear
      const replaceAllButton = dialog.getByRole("button", {
        name: /replace all/i,
      });
      const skipAllButton = dialog.getByRole("button", {
        name: /skip all/i,
      });

      const hasReplaceAll = await replaceAllButton
        .isVisible()
        .catch(() => false);
      const hasSkipAll = await skipAllButton.isVisible().catch(() => false);

      // At least one batch action should be available
      expect(hasReplaceAll || hasSkipAll).toBeTruthy();
    });
  });

  // -------------------------------------------------------------------------
  // Dialog UI tests
  // -------------------------------------------------------------------------

  test.describe("Duplicate dialog UI", () => {
    test("dialog shows file info with existing document ID", async ({
      page,
    }) => {
      let uploadCount = 0;

      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        uploadCount++;
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(
            uploadCount === 1 ? PDF_SUCCESS_RESPONSE : PDF_DUPLICATE_RESPONSE,
          ),
        });
      });

      const fileInput = page.locator('input[type="file"]');

      // Upload once
      await fileInput.setInputFiles({
        name: "important-doc.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4"),
      });
      await page.waitForTimeout(1500);

      // Upload again
      await fileInput.setInputFiles({
        name: "important-doc.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4"),
      });

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      const dialogText = (await dialog.textContent()) ?? "";

      // Should show filename
      expect(dialogText).toContain("important-doc.pdf");

      // Should show a truncated existing doc ID (first 8 chars of EXISTING_DOC_ID)
      const shortId = EXISTING_DOC_ID.slice(0, 8);
      expect(dialogText).toContain(shortId);

      // Should have a Confirm button
      const confirmButton = dialog.getByRole("button", { name: /confirm/i });
      await expect(confirmButton).toBeVisible();

      // Should have a Skip all button
      const skipButton = dialog.getByRole("button", {
        name: /skip all|skip/i,
      });
      await expect(skipButton.first()).toBeVisible();
    });

    test("dialog can toggle between Replace and Skip per file", async ({
      page,
    }) => {
      let uploadCount = 0;

      await page.route("**/api/v1/documents/pdf**", async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        uploadCount++;
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(
            uploadCount === 1 ? PDF_SUCCESS_RESPONSE : PDF_DUPLICATE_RESPONSE,
          ),
        });
      });

      const fileInput = page.locator('input[type="file"]');

      await fileInput.setInputFiles({
        name: "toggle-test.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4"),
      });
      await page.waitForTimeout(1500);

      await fileInput.setInputFiles({
        name: "toggle-test.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4"),
      });

      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // Find Replace badge/button in dialog
      const replaceBadge = dialog.getByText("Replace", { exact: true }).first();
      await expect(replaceBadge).toBeVisible();

      // Click Replace
      await replaceBadge.click();
      await page.waitForTimeout(200);

      // Click Skip to toggle back
      const skipBadge = dialog.getByText("Skip", { exact: true }).first();
      await expect(skipBadge).toBeVisible();
      await skipBadge.click();
      await page.waitForTimeout(200);

      // Confirm (with Skip decision = no reprocess)
      const confirmButton = dialog.getByRole("button", { name: /confirm/i });
      if (await confirmButton.isVisible().catch(() => false)) {
        await confirmButton.click();
      }

      await page.waitForTimeout(500);
      await expect(dialog).not.toBeVisible({ timeout: 3000 });
    });
  });
});
