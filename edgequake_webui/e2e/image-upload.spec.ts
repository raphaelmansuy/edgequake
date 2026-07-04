/**
 * Image upload E2E — verifies UI routes PNG to multipart /documents/upload.
 *
 * @implements FEAT0203 - Image document upload
 */
import { test, expect } from "@playwright/test";
import { mockBackendForUiOnly } from "./helpers/mock-backend";
import { uploadFilesOnDocumentsPage } from "./helpers/upload";

const TINY_PNG_BASE64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

test.describe("Image upload routing", () => {
  test("PNG upload hits multipart /documents/upload, not JSON /documents", async ({
    page,
  }) => {
    await mockBackendForUiOnly(page);

    let uploadEndpointHit = false;
    let jsonDocumentsHit = false;

    await page.route("**/api/v1/documents/upload", async (route) => {
      uploadEndpointHit = true;
      const contentType = route.request().headers()["content-type"] ?? "";
      expect(contentType).toContain("multipart/form-data");
      await route.fulfill({
        status: 202,
        contentType: "application/json",
        body: JSON.stringify({
          document_id: "e2e-img-doc-001",
          status: "pending",
          track_id: "track-e2e-img",
          task_id: "task-e2e-img",
          filename: "spec026-e2e.png",
          size: 68,
          content_hash: "abc",
          chunk_count: 0,
          entity_count: 0,
          relationship_count: 0,
          is_duplicate: false,
        }),
      });
    });

    await page.route("**/api/v1/documents", async (route) => {
      if (route.request().method() === "POST") {
        jsonDocumentsHit = true;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ items: [], total: 0 }),
      });
    });

    await uploadFilesOnDocumentsPage(page, {
      name: "spec026-e2e.png",
      mimeType: "image/png",
      buffer: Buffer.from(TINY_PNG_BASE64, "base64"),
    });

    await expect
      .poll(() => uploadEndpointHit, { timeout: 15_000 })
      .toBe(true);
    expect(jsonDocumentsHit).toBe(false);
  });
});
