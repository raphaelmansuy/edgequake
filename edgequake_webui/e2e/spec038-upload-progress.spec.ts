/**
 * SPEC-038 — Honest upload byte progress E2E (REQ-038-11)
 */

import { expect, test } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";
import { mockSpec038AdmissionRoutes } from "./helpers/spec038-admission-mocks";
import { spec038Screenshot } from "./helpers/screenshot-paths";

function buildLargePageCountPdf(pageCount: number): Buffer {
  const body = `%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count ${pageCount}/Kids[]>>endobj
xref
0 3
trailer<</Size 3/Root 1 0 R>>
startxref
100
%%EOF`;
  return Buffer.from(body, "utf8");
}

function buildPaddedPdf(pageCount: number, targetBytes: number): Buffer {
  const header = buildLargePageCountPdf(pageCount);
  if (header.length >= targetBytes) return header;
  const pad = Buffer.alloc(targetBytes - header.length, 0x20);
  return Buffer.concat([header, pad]);
}

test.describe("SPEC-038 Upload byte progress", () => {
  test.setTimeout(90_000);

  test.beforeEach(async ({ page }) => {
    await mockSpec038AdmissionRoutes(page);
    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
  });

  test("shows MB counter during PDF upload (no admission)", async ({ page }) => {
    await page.route("**/api/v1/documents/pdf", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      await new Promise((r) => setTimeout(r, 400));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "pdf-spec038-progress",
          status: "queued",
          task_id: "task-spec038-progress",
          track_id: "upload-spec038-progress",
          duplicate_of: null,
        }),
      });
    });

    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-progress-"));
    const fixturePath = path.join(tmpDir, "medium-doc.pdf");
    fs.writeFileSync(fixturePath, buildPaddedPdf(50, 512 * 1024));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);

    const bytesLabel = page.getByTestId("spec038-upload-bytes-sent");
    await expect(bytesLabel).toBeVisible({ timeout: 15_000 });
    await expect(bytesLabel).toContainText(/Sending|Saving/i);

    await page.screenshot({
      path: spec038Screenshot("05-upload-byte-progress.png"),
      fullPage: false,
      animations: "disabled",
    });
  });

  test("admission confirm shows transfer then saving labels", async ({ page }) => {
    let uploadBody = "";
    await page.route("**/api/v1/documents/pdf", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      uploadBody = route.request().postData() ?? "";
      await new Promise((r) => setTimeout(r, 300));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "pdf-spec038-admit-progress",
          status: "queued",
          task_id: "task-spec038-admit",
          track_id: "upload-spec038-admit",
          duplicate_of: null,
        }),
      });
    });

    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-admit-progress-"));
    const fixturePath = path.join(tmpDir, "large-admit.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(250));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);
    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeVisible();
    await page.getByTestId("spec038-admission-confirm").click();

    await expect
      .poll(() => uploadBody, { timeout: 20_000 })
      .toContain("edgeparse");

    await expect(page.getByTestId("spec038-upload-bytes-sent")).toBeVisible({
      timeout: 10_000,
    });

    await page.screenshot({
      path: spec038Screenshot("06-admission-upload-progress.png"),
      fullPage: false,
      animations: "disabled",
    });
  });
});
