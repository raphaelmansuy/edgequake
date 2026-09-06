/**
 * SPEC-146 — break-glass banner when active session exists.
 * Human-readable expiry; no ISO-only dump / deny codes.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";

test.describe("SPEC-146 break-glass", () => {
  test("banner visible for active session; no deny-reason leak", async ({ page }) => {
    const expires = new Date(Date.now() + 10 * 60_000).toISOString();
    await setupSpec146Ui(page, {
      sessions: [
        {
          session_id: "bg-active-1",
          workspace_id: "ws",
          principal_kind: "user",
          principal_id: "admin",
          reason: "incident response",
          expires_at: expires,
          created_at: new Date().toISOString(),
        },
      ],
    });
    await page.goto("/documents", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.getByTestId("document-dropzone")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("spec146-break-glass-banner")).toBeVisible({
      timeout: 20_000,
    });
    const banner = await page.getByTestId("spec146-break-glass-banner").innerText();
    expect(banner.toLowerCase()).toContain("break-glass");
    expect(banner.toLowerCase()).toContain("audited");
    expect(banner.toLowerCase()).toMatch(/until/);
    expect(banner.toLowerCase()).not.toContain("not_in_allow_set");
    expect(banner.toLowerCase()).not.toContain("n hidden");
    expect(banner.toLowerCase()).not.toContain("restricted");
    // Prefer locale datetime over raw ISO dump
    expect(banner).not.toMatch(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
  });
});
