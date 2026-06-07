/**
 * SPEC-017 — Full pipeline proof via live API + UI.
 *
 * Proves: POST /documents (sync) → chunk → extract → graph storage.
 * Screenshots: specs/017-dry-and-solid-audit/005-edgequake-pipeline/e2e/screenshots/
 */
import fs from "node:fs";
import os from "node:os";
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
  "../../specs/017-dry-and-solid-audit/005-edgequake-pipeline/e2e/screenshots",
);

const PIPELINE_DOC = `
EdgeQuake is a Rust RAG framework for knowledge graph construction.
Sarah Chen designed EdgeQuake to integrate Apache AGE and pgvector.
Michael Torres leads LLM integration for entity extraction pipelines.
John Smith contributed the Axum REST API for document ingestion.
`.trim();

/** Workspace with mock LLM — proves chunk path; live mock has no pre-seeded JSON. */
async function createMockPipelineWorkspace(
  request: import("@playwright/test").APIRequestContext,
  label: string,
) {
  const suffix = Date.now();
  const tenantRes = await request.post(`${BACKEND_URL}/api/v1/tenants`, {
    data: { name: `${label} tenant ${suffix}` },
  });
  expect(tenantRes.ok()).toBeTruthy();
  const tenant = (await tenantRes.json()) as { id: string };

  const wsRes = await request.post(
    `${BACKEND_URL}/api/v1/tenants/${tenant.id}/workspaces`,
    {
      data: {
        name: `${label} mock ws ${suffix}`,
        slug: `${label}-mock-${suffix}`.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
        llm_provider: "mock",
        llm_model: "mock-model",
        embedding_provider: "mock",
        embedding_model: "mock-embedding",
        embedding_dimension: 1536,
        entity_types: ["PERSON", "ORGANIZATION", "TECHNOLOGY", "CONCEPT"],
      },
    },
  );
  expect(wsRes.ok()).toBeTruthy();
  const ws = (await wsRes.json()) as { id: string };
  return { tenantId: tenant.id, workspaceId: ws.id };
}

async function pollDocumentStatus(
  request: import("@playwright/test").APIRequestContext,
  docId: string,
  tenantId: string,
  workspaceId: string,
  maxMs = 120_000,
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
      // Async tasks use pending/processing until terminal
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
  const boundary = `spec017-pdf-${Date.now()}`;
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

/** Poll PDF task until markdown + document_id linked (pdf_conversion → text_insert). */
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

test.describe("@audit SPEC-017 full pipeline ingestion @audit", () => {
  test("sync text upload completes chunk + extract pipeline (Mistral workspace)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();

    const ctx = await createTenantWorkspaceViaApi(
      request,
      "spec017-full-pipeline",
    );
    await seedTenantStoreOnPage(page, ctx);

    const upload = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title: `spec017-pipeline-${Date.now()}.md`,
        content: PIPELINE_DOC,
        async_processing: false,
      },
      timeout: 180_000,
    });

    expect([200, 201]).toContain(upload.status());
    const body = (await upload.json()) as {
      document_id?: string;
      id?: string;
      status?: string;
      chunk_count?: number;
      entity_count?: number;
    };

    const docId = body.document_id ?? body.id;
    expect(docId).toBeTruthy();
    expect(body.status).toMatch(/processed|completed/i);
    expect((body.chunk_count ?? 0) > 0).toBeTruthy();
    expect((body.entity_count ?? 0) > 0).toBeTruthy();

    await gotoApp(page, "/documents");
    await expect(
      page.getByRole("heading", { name: /Documents/i }).first(),
    ).toBeVisible({ timeout: 15_000 });

    await page.screenshot({
      path: path.join(ARTIFACT_DIR, "03-sync-upload-completed.png"),
      fullPage: false,
    });

    const docRow = page.getByText(/spec017-pipeline/i).first();
    await expect(docRow).toBeVisible({ timeout: 30_000 });

    await page.locator("main").first().screenshot({
      path: path.join(ARTIFACT_DIR, "04-documents-after-full-pipeline.png"),
    });

    const meta = await pollDocumentStatus(
      request,
      docId!,
      ctx.tenantId,
      ctx.workspaceId,
      5_000,
    );
    expect((meta.entity_count ?? 0) > 0).toBeTruthy();
  });

  test("mock workspace sync upload proves chunk stage (extraction may partial-fail live)", async ({
    request,
  }) => {
    skipUnlessLiveStack();

    const { tenantId, workspaceId } = await createMockPipelineWorkspace(
      request,
      "spec017-mock-chunk",
    );

    const upload = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(tenantId, workspaceId),
      data: {
        title: `spec017-mock-${Date.now()}.md`,
        content: PIPELINE_DOC,
        async_processing: false,
      },
      timeout: 180_000,
    });

    expect([200, 201]).toContain(upload.status());
    const body = (await upload.json()) as {
      document_id?: string;
      chunk_count?: number;
      status?: string;
    };
    expect((body.chunk_count ?? 0) > 0).toBeTruthy();
    // Live mock provider has no queued JSON — chunking proven; extraction may be empty.
    expect(body.status).toMatch(/processed|completed|partial/i);
  });

  test("UI file upload triggers async pipeline through chunking stage", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();
    await bootstrapDeterministicUiContext(page, request, "spec017-ui-upload");

    const tmpFile = path.join(
      os.tmpdir(),
      `spec017-pipeline-${Date.now()}.md`,
    );
    fs.writeFileSync(tmpFile, PIPELINE_DOC);

    try {
      await gotoApp(page, "/documents");
      const fileInput = page.locator('input[type="file"]').first();
      await fileInput.setInputFiles(tmpFile);

      await expect(
        page.getByText(/uploaded|processing|chunking|extracting/i).first(),
      ).toBeVisible({ timeout: 60_000 });

      await page.screenshot({
        path: path.join(ARTIFACT_DIR, "05-ui-upload-chunking-stage.png"),
        fullPage: false,
      });

      await page.locator("main").first().screenshot({
        path: path.join(ARTIFACT_DIR, "06-ui-upload-processing-panel.png"),
      });
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  test("async API upload polls to Completed (background task pipeline)", async ({
    page,
    request,
  }) => {
    skipUnlessLiveStack();

    const ctx = await createTenantWorkspaceViaApi(
      request,
      "spec017-async-pipeline",
    );

    const upload = await request.post(`${BACKEND_URL}/api/v1/documents`, {
      headers: tenantHeaders(ctx.tenantId, ctx.workspaceId),
      data: {
        title: `spec017-async-${Date.now()}.md`,
        content: PIPELINE_DOC,
        async_processing: true,
      },
      timeout: 60_000,
    });

    expect([200, 201, 202]).toContain(upload.status());
    const body = (await upload.json()) as {
      document_id?: string;
      id?: string;
      status?: string;
    };
    const docId = body.document_id ?? body.id;
    expect(docId).toBeTruthy();

    const meta = await pollDocumentStatus(
      request,
      docId!,
      ctx.tenantId,
      ctx.workspaceId,
      300_000,
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
      path: path.join(ARTIFACT_DIR, "07-async-pipeline-completed.png"),
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

    const ctx = await createTenantWorkspaceViaApi(
      request,
      "spec017-pdf-pipeline",
    );
    const pdfBytes = fs.readFileSync(pdfPath);
    const trackId = `spec017-pdf-${Date.now()}`;
    const { boundary, body } = pdfMultipartBody("001_simple_text.pdf", pdfBytes, {
      title: `spec017-pdf-${Date.now()}`,
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
