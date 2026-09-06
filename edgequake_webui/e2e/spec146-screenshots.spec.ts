/**
 * SPEC-146 — capture desktop / tablet / mobile PNGs into
 * specs/146-rbac-attributes-based-securty/e2e/screnshist/ (KEEP SPELLING).
 *
 * Real product chrome only — fails on TenantGuard. No fixtureHtml.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { spec146Screenshot } from "./helpers/screenshot-paths";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";
import {
  assertDropzoneContainment,
  assertSecurityFieldsUsable,
} from "./helpers/spec146-visibility";
import { ZERO_AUTHZ_ANSWER } from "../src/lib/query/query-empty-copy";

const VIEWPORTS = [
  { name: "desktop-1440", width: 1440, height: 900 },
  { name: "tablet-768", width: 768, height: 1024 },
  { name: "mobile-375", width: 375, height: 812 },
] as const;

async function capture(
  page: import("@playwright/test").Page,
  fileName: string,
) {
  await page.screenshot({
    path: spec146Screenshot(fileName),
    fullPage: true,
  });
}

test.describe("SPEC-146 screenshots", () => {
  for (const vp of VIEWPORTS) {
    test(`documents + banner @ ${vp.name}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      const expires = new Date(Date.now() + 10 * 60_000).toISOString();
      await setupSpec146Ui(page, {
        documents: [
          {
            id: "doc-public-1",
            title: "Public Handbook",
            file_name: "public.pdf",
            status: "completed",
            classification: "internal",
            share_mode: "workspace",
            owner_principal_id: "alice-user-id",
            created_at: "2026-01-01T00:00:00Z",
          },
        ],
        sessions: [
          {
            session_id: "bg-active-1",
            workspace_id: "ws",
            principal_kind: "user",
            principal_id: "admin",
            reason: "incident",
            expires_at: expires,
            created_at: new Date().toISOString(),
          },
        ],
      });
      await page.goto("/documents", GOTO_OPTS);
      await assertProductChrome(page);
      await expect(page.getByTestId("document-dropzone")).toBeVisible({
        timeout: 20_000,
      });
      await expect(page.getByTestId("spec146-break-glass-banner")).toBeVisible({
        timeout: 20_000,
      });
      const banner = (
        await page.getByTestId("spec146-break-glass-banner").innerText()
      ).toLowerCase();
      expect(banner).toContain("break-glass");
      expect(banner).toContain("audited");
      const main = page.locator("main");
      await expect(main).not.toContainText("SPEC-146");
      await expect(main).not.toContainText("NaN");
      await expect(main).not.toContainText("SPEC038");
      // Cards below lg (1024); dense table from lg up (tablet-768 must be cards).
      if (vp.width < 1024) {
        await expect(page.getByTestId("spec146-doc-cards")).toBeVisible({
          timeout: 20_000,
        });
        const card = page.getByTestId("spec146-doc-card").first();
        await expect(card).toContainText("alice", { timeout: 20_000 });
        await expect(card).toContainText(/Internal/i);
        await expect(card).toContainText(/Workspace/i);
        const headerText = await page.locator("main").innerText();
        expect(headerText).not.toMatch(/\bClas\b/);
        expect(headerText).not.toMatch(/\bSha\b/);
        expect(headerText).not.toMatch(/\bOwn\b/);
      } else {
        await expect(page.getByTestId("spec146-cell-owner")).toContainText("alice");
      }
      await capture(page, `documents-breakglass-${vp.name}.png`);
    });

    test(`settings authz @ ${vp.name}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await setupSpec146Ui(page, {
        members: [
          {
            workspace_id: "ws",
            principal_kind: "user",
            principal_id: "alice-user-id",
            role_id: "role-1",
            role_name: "viewer",
          },
          {
            workspace_id: "ws",
            principal_kind: "user",
            principal_id: "bob-user-id",
            role_id: "role-editor",
            role_name: "editor",
          },
        ],
      });
      await page.goto("/settings", GOTO_OPTS);
      await assertProductChrome(page);
      const authz = page.getByTestId("spec146-authz-settings");
      await expect(authz).toBeVisible({ timeout: 20_000 });
      await expect(page.getByTestId("spec146-members-skeleton")).toHaveCount(0, {
        timeout: 20_000,
      });
      await expect(page.getByTestId("spec146-members-list")).toBeVisible({
        timeout: 20_000,
      });
      await expect(page.getByTestId("spec146-members-list")).toContainText(/alice/i, {
        timeout: 20_000,
      });
      await expect(page.getByTestId("spec146-members-list")).toContainText(/bob/i);
      await expect(page.getByTestId("spec146-member-user")).toBeEnabled();
      const main = page.locator("main");
      await expect(main).not.toContainText("SPEC-146");
      await expect(main).not.toContainText("SPEC038");
      await expect(page.getByTestId("spec146-members-list")).not.toContainText(
        "Loading",
      );
      // Roles summary must be human labels, not raw document: slugs (default path).
      await expect(authz).toContainText("List documents");
      await expect(authz).not.toContainText("document:list_meta");
      // First-class Attributes + Break-glass (004-ux IA).
      await expect(page.getByTestId("spec146-attrs-card")).toBeVisible();
      await expect(page.getByTestId("spec146-break-glass-card")).toBeVisible();
      await expect(page.getByTestId("spec146-authz-advanced")).toHaveCount(0);
      await expect(page.getByTestId("spec146-policy-advanced-toggle")).toBeVisible();
      await expect(page.getByTestId("spec146-roles-create-form")).toHaveCount(0);
      await expect(page.getByTestId("spec146-roles-create-toggle")).toBeVisible();
      await expect(page.getByTestId("spec146-members-list")).toContainText(/secret/i);
      await expect(page.getByTestId("spec146-members-list")).toContainText(/internal/i);
      await expect(page.getByTestId("spec146-members-list")).toContainText(/department:eng/i);
      await expect(page.getByTestId("spec146-member-invite-hint")).toBeVisible();
      // Scroll Attrs + Break-glass into view for evidence (not Members-only).
      await page.getByTestId("spec146-attrs-card").scrollIntoViewIfNeeded();
      await expect(page.getByTestId("spec146-attrs-card")).toBeInViewport();
      await page.getByTestId("spec146-break-glass-card").scrollIntoViewIfNeeded();
      await expect(page.getByTestId("spec146-break-glass-card")).toBeInViewport();
      if (vp.width < 1024) {
        await expect(page.getByTestId("spec146-members-cards")).toBeVisible();
      }
      if (vp.width <= 375) {
        const box = await authz.evaluate((el) => ({
          scrollWidth: el.scrollWidth,
          clientWidth: el.clientWidth,
        }));
        expect(box.scrollWidth).toBeLessThanOrEqual(box.clientWidth + 1);
      }
      await capture(page, `settings-authz-${vp.name}.png`);
    });

    test(`query empty @ ${vp.name}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await setupSpec146Ui(page, { documents: [] });
      await page.goto("/query", GOTO_OPTS);
      await assertProductChrome(page);
      await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
      if (vp.width < 1024) {
        await expect(page.getByTestId("query-header-subtitle")).toBeHidden();
      }
      const input = page.getByRole("textbox", { name: /ask a question/i });
      await expect(input).toBeVisible({ timeout: 20_000 });
      await input.fill("ENTITY_X?");
      const send = page.getByRole("button", { name: /send/i });
      await expect(send).toBeEnabled();
      await send.click();
      await expect(page.getByText(ZERO_AUTHZ_ANSWER).first()).toBeVisible({
        timeout: 15_000,
      });
      await expect(page.getByTestId("spec146-zero-authz-help")).toBeVisible();
      // Idle composer: Send visible, Stop gone after stream done.
      await expect(page.getByRole("button", { name: /^send$/i })).toBeVisible({
        timeout: 10_000,
      });
      await expect(
        page.getByRole("button", { name: /stop generating/i }),
      ).toHaveCount(0);
      // History list reflects the streamed turn when desktop/tablet panel is mounted.
      if (vp.width >= 768) {
        await expect(page.getByText("No conversations yet")).toHaveCount(0);
        await expect(page.getByText(/2 messages/i).first()).toBeVisible({
          timeout: 10_000,
        });
      }      const main = page.locator("main");
      await expect(main).not.toContainText("SPEC038");
      await expect(main).not.toContainText("Restricted");
      await capture(page, `query-empty-${vp.name}.png`);
    });

    test(`upload labels @ ${vp.name}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await setupSpec146Ui(page);
      await page.goto("/documents", GOTO_OPTS);
      await assertProductChrome(page);
      await assertSecurityFieldsUsable(page);
      await assertDropzoneContainment(page);
      const main = page.locator("main");
      await expect(main).not.toContainText("NaN");
      await expect(main).not.toContainText("SPEC038");
      await capture(page, `upload-labels-${vp.name}.png`);
    });
  }
});
