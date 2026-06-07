/**
 * SPEC-017 edgequake-api — Playwright UI proof for API-backed routes + full pipeline.
 * Writes PNG to specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/screenshots/
 *
 * Requires live stack: E2E_LIVE_STACK=1 (see run_playwright_proof.sh).
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test } from "@playwright/test";
import { BACKEND_URL } from "./helpers/backend-url";
import {
  bootstrapDeterministicUiContext,
  seedTenantStoreOnPage,
} from "./helpers/bootstrap-ui";
import { skipUnlessLiveStack } from "./helpers/live-stack";
import { gotoApp } from "./helpers/navigation";
import {
  createTenantWorkspaceViaApi,
  tenantHeaders,
} from "./helpers/spec013-api";

const ARTIFACT_DIR = path.resolve(
  __dirname,
  "../../specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/screenshots",
);

const API_SYNC_DOC = `
EdgeQuake API routes document ingestion through WorkspacePipelineFactory.
Sarah Chen works at EDGEQUAKE on the Axum REST layer.
Michael Torres leads LLM integration for entity extraction pipelines.
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
      const body = (await res.json()) as {
        status?: string;
        entity_count?: number;
        chunk_count?: number;
      };
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

function pdfMultipartBody(
  filename: string,
  pdfBytes: Buffer,
  fields: Record<string, string>,
): { boundary: string; body: Buffer } {
  const boundary = `spec017-api-pdf-${Date.now()}`;
  const chunks: Buffer[] = [];
  for (const [k, v] of Object.entries(fields)) {
    chunks.push(Buffer.from(`--${boundary}\r\n`));
    chunks.push(
      Buffer.from(`Content-Disposition: form-data; name="${k}"\r\n\r\n${v}\r\n`),
    );
  }
  chunks.push(Buffer.from(`--${boundary}\r\n`));
  chunks.push(
    Buffer.from(
      `Content-Disposition: form-data; name="file"; filename="${filename}"\r\nContent-Type: application/pdf\r\n\r\n`,
    ),
  );
  chunks.push(pdfBytes);
  chunks.push(Buffer.from(`\r\n--${boundary}--\r\n`));
  return { boundary, body: Buffer.concat(chunks) };
}

async function pollPdfCompleted(
  request: import("@playwright/test").APIRequestContext,
  pdfId: string,
  tenantId: string,
  workspaceId: string,
  maxMs = 600_000,
) {
  const deadline = Date.now() + maxMs;
  while (Date.now() < deadline) {
    const res = await request.get(`${BACKEND_URL}/api/v1/documents/pdf/${pdfId}`, {
      headers: tenantHeaders(tenantId, workspaceId),
    });
    if (res.ok()) {
      const body = (await res.json()) as {
        status?: string;
        document_id?: string | null;
      };
      const status = (body.status ?? "").toLowerCase();
      if (status === "failed") {
        throw new Error(`PDF pipeline failed: ${JSON.stringify(body)}`);
      }
      if (status === "completed" && (body.document_id?.length ?? 0) > 10) {
        return body;
      }
    }
    await new Promise((r) => setTimeout(r, 3000));
  }
  throw new Error(`PDF ${pdfId} did not complete within ${maxMs}ms`);
}

test.describe("@audit SPEC-017 API query + documents UI @audit", () => {
  test("health endpoint returns healthy JSON", async ({ request }) => {
    skipUnlessLiveStack();
    const res = await request.get(`${BACKEND_URL}/health`);
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.status).toBe("healthy");
    expect(body.storage_mode).toBeTruthy();
    expect(body.components).toBeTruthy();
  });

  test("query page renders SOTA query UI", async ({ page, request }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "spec017-api-query");

    await gotoApp(page, "/query");

    const queryInput = page.locator("textarea.query-input").first();
    await expect(queryInput).toBeVisible({ timeout: 15_000 });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "01-query-page-header.png"),
      fullPage: false,
    });

    const mainPanel = page.locator("main").first();
    await mainPanel.screenshot({
      path: path.join(ARTIFACT_DIR, "02-query-main-panel.png"),
    });
  });

  test("documents page renders upload zone (ingestion API entry)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "spec017-api-docs");

    await gotoApp(page, "/documents");

    await expect(page.getByText("Documents").first()).toBeVisible({
      timeout: 15_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "03-documents-page-header.png"),
      fullPage: false,
    });

    const mainPanel = page.locator("main").first();
    await mainPanel.screenshot({
      path: path.join(ARTIFACT_DIR, "04-documents-main-panel.png"),
    });
  });

  test("sync document upload completes via API pipeline", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();

    const ctx = await createTenantWorkspaceViaApi(request, "spec017-api-sync");
    await seedTenantStoreOnPage(page, ctx);

    const title = `spec017-api-sync-${Date.now()}.md`;
    const uploadRes = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title,
        content: API_SYNC_DOC,
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
      timeout: 60_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "05-sync-upload-completed.png"),
      fullPage: false,
    });
  });

  test("async API upload polls to Completed (background task pipeline)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    test.setTimeout(360_000);

    const ctx = await createTenantWorkspaceViaApi(
      request,
      "spec017-api-async",
    );

    const title = `spec017-api-async-${Date.now()}.md`;
    const upload = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title,
        content: API_SYNC_DOC,
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
    expect((meta.entity_count ?? 0) > 0).toBeTruthy();
    expect((meta.chunk_count ?? 0) > 0).toBeTruthy();

    await seedTenantStoreOnPage(page, ctx);
    await gotoApp(page, "/documents");
    await expect(page.getByText(/Completed/i).first()).toBeVisible({
      timeout: 30_000,
    });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "06-async-pipeline-completed.png"),
      fullPage: false,
    });
  });

  test("PDF upload via text parser reaches Completed (pdf_conversion → text_insert)", async ({
    request,
  }) => {
    skipUnlessLiveStack();
    test.setTimeout(600_000);

    const pdfPath = path.resolve(
      __dirname,
      "../../legacy/edgequake-pdf/test-data/001_simple_text.pdf",
    );
    test.skip(!fs.existsSync(pdfPath), "PDF fixture missing");

    const ctx = await createTenantWorkspaceViaApi(request, "spec017-api-pdf");
    const pdfBytes = fs.readFileSync(pdfPath);
    const trackId = `spec017-api-pdf-${Date.now()}`;
    const { boundary, body } = pdfMultipartBody("001_simple_text.pdf", pdfBytes, {
      title: `spec017-api-pdf-${Date.now()}`,
      enable_vision: "false",
      pdf_parser_backend: "text",
      force_reindex: "true",
      track_id: trackId,
    });

    const upload = await request.fetch(`${BACKEND_URL}/api/v1/documents/pdf`, {
      method: "POST",
      headers: {
        ...tenantHeaders(ctx.tenantId, ctx.workspaceId),
        "Content-Type": `multipart/form-data; boundary=${boundary}`,
      },
      data: body,
      timeout: 120_000,
    });

    expect([200, 201, 202]).toContain(upload.status());
    const uploadBody = (await upload.json()) as { pdf_id?: string };
    expect(uploadBody.pdf_id).toBeTruthy();

    const completed = await pollPdfCompleted(
      request,
      uploadBody.pdf_id!,
      ctx.tenantId,
      ctx.workspaceId,
    );
    expect(completed.document_id).toBeTruthy();

    const meta = await pollDocumentStatus(
      request,
      completed.document_id!,
      ctx.tenantId,
      ctx.workspaceId,
      120_000,
    );
    expect(meta.status).toMatch(/processed|completed/i);
    expect((meta.chunk_count ?? 0) > 0).toBeTruthy();
  });
});
