/**
 * SPEC-146 — existence-hiding: unauthorized titles omitted; no Restricted grey rows.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";

test.describe("SPEC-146 existence-hiding", () => {
  test("list shows only authorized docs; no Restricted / hidden-count copy", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page, {
      documents: [
        {
          id: "doc-public-1",
          title: "Public Handbook",
          file_name: "public.pdf",
          status: "completed",
          classification: "internal",
          share_mode: "workspace",
          security_status: "ok",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        },
      ],
    });
    await page.goto("/documents", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
    await expect(
      page.getByText(/Public Handbook/i).filter({ visible: true }),
    ).toBeVisible({
      timeout: 20_000,
    });

    const main = (await page.locator("main").innerText()).toLowerCase();
    expect(main).not.toContain("restricted");
    expect(main).not.toContain("n hidden");
    expect(main).not.toContain("hidden docs");
    expect(main).not.toContain("not_in_allow_set");
    expect(main).toContain("public handbook");
  });
});
