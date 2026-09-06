/**
 * SPEC-146 — quarantine chip + Retry labels recovery journey.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";

test.describe("SPEC-146 quarantine", () => {
  test("shows quarantine chip and Retry labels recovery", async ({ page }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page, {
      documents: [
        {
          id: "doc-q-1",
          title: "Quarantined Handbook",
          file_name: "q.pdf",
          status: "completed",
          classification: "secret",
          share_mode: "classified",
          security_status: "quarantined",
          owner_principal_id: "alice-user-id",
          created_at: "2026-01-01T00:00:00Z",
        },
      ],
    });
    await page.goto("/documents", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(
      page.getByTestId("spec146-quarantine-chip").filter({ visible: true }),
    ).toBeVisible({
      timeout: 20_000,
    });
    const retry = page.getByTestId("spec146-retry-labels").filter({ visible: true });
    await expect(retry).toBeVisible();
    await retry.click();
    await expect(page.getByTestId("spec146-retry-labels-dialog")).toBeVisible();
    await page.getByTestId("spec146-retry-labels-submit").click();
    await expect(
      page.getByTestId("spec146-quarantine-chip").filter({ visible: true }),
    ).toHaveCount(0, {
      timeout: 15_000,
    });
    await expect(page.locator("main")).not.toContainText("Restricted");
  });
});
