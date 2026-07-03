/**
 * SPEC-038 — Large PDF admission dialog E2E
 * @implements REQ-038-04, UX-038-01..05
 */

import { expect, test } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";
import { mockSpec038AdmissionRoutes, seedSpec038TenantContext } from "./helpers/spec038-admission-mocks";
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

test.describe("SPEC-038 Large PDF Admission", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
  });

  test("shows admission dialog for 603-page PDF fixture", async ({ page }) => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-"));
    const fixturePath = path.join(tmpDir, "large-guide-stub.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(603));

    await page.screenshot({
      path: spec038Screenshot("01-documents-before-upload.png"),
      fullPage: false,
    });

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);

    const dialog = page.getByTestId("spec038-large-pdf-admission-dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId("spec038-admission-summary")).toContainText("603");
    await expect(page.getByTestId("spec038-admission-recommendation")).toBeVisible();
    await expect(page.getByTestId("spec038-admission-eta-edgeparse")).toBeVisible();
    await expect(page.getByTestId("spec038-parser-choice")).toBeVisible();

    await page.screenshot({
      path: spec038Screenshot("02-admission-dialog-edgeparse-recommended.png"),
      fullPage: false,
    });

    await page.getByTestId("spec038-admission-cancel").click();
    await expect(dialog).toBeHidden({ timeout: 10_000 });
  });

  test("confirm uploads PDF with edgeparse parser override", async ({ page }) => {
    let uploadHasEdgeparse = false;
    await page.route("**/api/v1/documents/pdf**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      const body = route.request().postDataBuffer();
      uploadHasEdgeparse = body?.toString().includes("edgeparse") ?? false;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "pdf-spec038-upload",
          status: "processing",
          task_id: "task-spec038-upload",
          track_id: "upload-spec038-upload",
          duplicate_of: null,
        }),
      });
    });

    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-"));
    const fixturePath = path.join(tmpDir, "large-guide-upload.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(603));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);
    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeVisible();
    await page.getByTestId("spec038-admission-confirm").click();

    await expect
      .poll(() => uploadHasEdgeparse, { timeout: 25_000 })
      .toBe(true);
  });

  test("vision parser shows slowdown warning", async ({ page }) => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-"));
    const fixturePath = path.join(tmpDir, "large-survey.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(250));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);

    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeVisible();
    await page.getByLabel(/Vision OCR/i).click();
    await expect(page.getByTestId("spec038-admission-eta-vision")).toBeVisible();

    await page.screenshot({
      path: spec038Screenshot("04-vision-slowdown-warning.png"),
      fullPage: false,
    });

    await page.getByTestId("spec038-admission-cancel").click();
    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeHidden();
  });

  test.fixme("skips admission when upload parser is EdgeParse", async ({ page }) => {
    await page.route("**/api/v1/documents/pdf**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "pdf-spec038-silent-upload",
          status: "queued",
          track_id: "upload-spec038-silent",
          duplicate_of: null,
        }),
      });
    });

    await page.getByTestId("spec038-upload-parser-select").click({ timeout: 10_000 });
    await page.getByRole("option", { name: "EdgeParse" }).click({ timeout: 10_000 });

    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-"));
    const fixturePath = path.join(tmpDir, "large-edgeparse-upload.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(603));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);

    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeHidden({
      timeout: 5_000,
    });

    await page.screenshot({
      path: spec038Screenshot("07-silent-upload-edgeparse-selected.png"),
      fullPage: false,
      animations: "disabled",
      timeout: 10_000,
    });
  });
});

test.describe("SPEC-038 Large PDF Admission — workspace EdgeParse default", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    await mockSpec038AdmissionRoutes(page, { workspacePdfParserBackend: "edgeparse" });
    await seedSpec038TenantContext(page, { workspacePdfParserBackend: "edgeparse" });
    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor({
      state: "visible",
      timeout: 20_000,
    });
  });

  test.fixme("skips admission when workspace default is EdgeParse", async ({ page }) => {
    await page.route("**/api/v1/documents/pdf**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.fallback();
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          pdf_id: "pdf-spec038-ws-edgeparse",
          status: "queued",
          track_id: "upload-spec038-ws",
          duplicate_of: null,
        }),
      });
    });

    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "spec038-"));
    const fixturePath = path.join(tmpDir, "large-ws-edgeparse.pdf");
    fs.writeFileSync(fixturePath, buildLargePageCountPdf(603));

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(fixturePath);

    await expect(page.getByTestId("spec038-large-pdf-admission-dialog")).toBeHidden({
      timeout: 5_000,
    });

    await page.screenshot({
      path: spec038Screenshot("08-silent-upload-workspace-edgeparse.png"),
      fullPage: false,
      animations: "disabled",
      timeout: 10_000,
    });
  });
});
