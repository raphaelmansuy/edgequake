/**
 * SPEC-017 edgequake-webui — Playwright proof for DRY/SOLID remediation.
 * Screenshots: specs/017-dry-and-solid-audit/013-edgequake-webui/001-audit/e2e/screenshots/
 *
 * @audit Requires E2E_LIVE_STACK=1 + make dev-bg (auth disabled uses default workspace).
 */
import path from "node:path";
import { expect, test } from "@playwright/test";
import { waitForBackendHealthy } from "./helpers/app-ready";
import { BACKEND_URL } from "./helpers/backend-url";
import { requiresLiveStack, skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";
import {
  createTenantWorkspaceViaApi,
  tenantHeaders,
} from "./helpers/spec013-api";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/013-edgequake-webui/001-audit/e2e/screenshots",
);

const WEBUI_SYNC_DOC = `
EdgeQuake WebUI consumes split API modules under lib/api/edgequake/.
Sarah Chen works on the Next.js query interface and document manager.
Michael Torres validates entity extraction badges in the documents table.
`.trim();

async function pollDocumentStatus(
  request: import("@playwright/test").APIRequestContext,
  docId: string,
  tenantId: string,
  workspaceId: string,
  maxMs = 300_000,
) {
  const deadline = Date.now() + maxMs;
  while (Date.now() < deadline) {
    const res = await request.get(`${BACKEND_URL}/api/v1/documents/${docId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
    });
    if (res.ok()) {
      const body = (await res.json()) as { status?: string };
      const status = (body.status ?? "").toLowerCase();
      if (
        ["processed", "completed", "failed", "partial"].some((s) =>
          status.includes(s),
        )
      ) {
        return body;
      }
      if (status === "pending" || status === "processing") {
        await new Promise((r) => setTimeout(r, 3000));
        continue;
      }
    }
    await new Promise((r) => setTimeout(r, 2000));
  }
  throw new Error(`document ${docId} did not reach terminal status within ${maxMs}ms`);
}

test.describe("SPEC-017 webui DRY/SOLID proof", () => {
  test.beforeAll(async () => {
    if (!requiresLiveStack) return;
    await waitForBackendHealthy(60);
  });

  test.beforeEach(async () => {
    if (!requiresLiveStack) return;
    await waitForBackendHealthy(15);
  });

  test("query page shows mode selector chips (live stack)", async ({ page }) => {
    skipUnlessLiveStack();

    await gotoApp(page, "/query");

    const retry = page.getByRole("button", { name: /retry/i });
    if (await retry.isVisible({ timeout: 3_000 }).catch(() => false)) {
      await retry.click();
    }

    const queryInput = page.getByPlaceholder(/Ask a question/i);
    await expect(queryInput).toBeVisible({ timeout: 30_000 });

    await expect(page.getByRole("button", { name: /Local/i }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /Hybrid/i }).first()).toBeVisible();

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "03-query-mode-selector.png"),
      fullPage: false,
    });

    await page.locator("main").first().screenshot({
      path: path.join(ARTIFACT_DIR, "04-query-main-panel.png"),
    });
  });

  test("documents page shows upload zone (live stack)", async ({ page }) => {
    skipUnlessLiveStack();

    await gotoApp(page, "/documents");
    await expect(page.getByText("Documents").first()).toBeVisible({
      timeout: 30_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "05-documents-upload-zone.png"),
      fullPage: false,
    });
  });

  test("sync upload shows Completed badge via EnhancedStatusBadge (live stack)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    test.setTimeout(240_000);

    const ctx = await createTenantWorkspaceViaApi(request, "spec017-webui-sync");
    const title = `spec017-webui-sync-${Date.now()}.md`;
    const uploadRes = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title,
        content: WEBUI_SYNC_DOC,
        async_processing: false,
      },
      timeout: 180_000,
    });

    expect([200, 201]).toContain(uploadRes.status());
    const doc = (await uploadRes.json()) as {
      chunk_count?: number;
      entity_count?: number;
      status?: string;
    };
    expect((doc.chunk_count ?? 0) > 0).toBeTruthy();
    expect((doc.entity_count ?? 0) > 0).toBeTruthy();
    expect(doc.status).toMatch(/processed|completed/i);

    await gotoApp(page, "/documents");
    await expect(page.getByText(title).first()).toBeVisible({
      timeout: 90_000,
    });
    await expect(page.getByText(/Completed/i).first()).toBeVisible({
      timeout: 30_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "06-sync-pipeline-completed.png"),
      fullPage: false,
    });
  });

  test("async upload polls to Completed (background task pipeline)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    test.setTimeout(360_000);

    const ctx = await createTenantWorkspaceViaApi(request, "spec017-webui-async");
    const title = `spec017-webui-async-${Date.now()}.md`;
    const upload = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title,
        content: WEBUI_SYNC_DOC,
        async_processing: true,
      },
      timeout: 60_000,
    });

    expect([200, 201, 202]).toContain(upload.status());
    const body = (await upload.json()) as {
      document_id?: string;
      id?: string;
    };
    const docId = body.document_id ?? body.id;
    expect(docId).toBeTruthy();

    const meta = await pollDocumentStatus(
      request,
      docId!,
      ctx.tenantId,
      ctx.workspaceId,
    );
    expect(meta.status).toMatch(/processed|completed/i);

    await seedTenantStoreOnPage(page, ctx);
    await gotoApp(page, "/documents");
    await expect(page.getByText(/Completed/i).first()).toBeVisible({
      timeout: 60_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "07-async-pipeline-completed.png"),
      fullPage: false,
    });
  });
});
