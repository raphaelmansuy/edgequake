/**
 * Document upload helpers — single place for file-input discovery (DRY).
 */
import type { Page } from "@playwright/test";
import { GOTO_OPTS, waitForAppReady } from "./app-ready";

/** Navigate to documents and return the hidden file input from the dropzone. */
export async function getDocumentsFileInput(page: Page) {
  await page.goto("/documents", GOTO_OPTS);
  await page.getByRole("heading", { name: "Documents" }).waitFor({
    state: "visible",
    timeout: 20_000,
  });
  const input = page.locator('input[type="file"]').first();
  await input.waitFor({ state: "attached", timeout: 10_000 });
  return input;
}

/** Set files on the documents page upload input. */
export async function uploadFilesOnDocumentsPage(
  page: Page,
  files: Parameters<
    ReturnType<Page["locator"]>["setInputFiles"]
  >[0],
): Promise<void> {
  const input = await getDocumentsFileInput(page);
  await input.setInputFiles(files);
}
