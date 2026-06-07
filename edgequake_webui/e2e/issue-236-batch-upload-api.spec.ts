import { skipUnlessLiveStack } from "./helpers/live-stack";
/**
 * SPEC-014 / GitHub #236 — batch upload API for document and PDF.
 */
import { expect, test } from '@playwright/test';
import { issueScreenshot } from "./helpers/screenshot-paths";
import fs from 'node:fs';
import path from 'node:path';
import {
  createTenantWorkspaceViaApi,
  SPEC013_BACKEND,
  tenantHeaders,
} from './helpers/spec013-api';


function multipartBody(
  files: { field: string; filename: string; contentType: string; content: Buffer | string }[]
): { boundary: string; body: Buffer } {
  const boundary = `batch-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const chunks: Buffer[] = [];
  for (const file of files) {
    chunks.push(Buffer.from(`--${boundary}\r\n`));
    chunks.push(
      Buffer.from(
        `Content-Disposition: form-data; name="${file.field}"; filename="${file.filename}"\r\n`
      )
    );
    chunks.push(Buffer.from(`Content-Type: ${file.contentType}\r\n\r\n`));
    chunks.push(typeof file.content === 'string' ? Buffer.from(file.content) : file.content);
    chunks.push(Buffer.from('\r\n'));
  }
  chunks.push(Buffer.from(`--${boundary}--\r\n`));
  return { boundary, body: Buffer.concat(chunks) };
}


test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe('Issue #236 batch upload API', () => {
  test('openapi exposes batch upload endpoints', async ({ page, request }) => {
    await page.setViewportSize({ width: 1600, height: 1200 });
    const openapiResp = await request.get(`${SPEC013_BACKEND}/api-docs/openapi.json`);
    expect(openapiResp.ok()).toBeTruthy();
    const openapi = await openapiResp.json();
    expect(openapi.paths['/api/v1/documents/upload/batch']).toBeTruthy();
    expect(openapi.paths['/api/v1/documents/pdf/batch']).toBeTruthy();

    await page.goto(`${SPEC013_BACKEND}/swagger-ui/`, { waitUntil: 'domcontentloaded' });
    await expect(page.getByText('/api/v1/documents/upload/batch')).toBeVisible();
    await expect(page.getByText('/api/v1/documents/pdf/batch')).toBeVisible();
    await page.screenshot({
      path: issueScreenshot("issue-236", "001-swagger-batch-endpoints.png"),
      fullPage: false,
    });
  });

  test('batch document upload ingests multiple text files in one request', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(request, 'issue-236-docs');
    const { boundary, body } = multipartBody([
      {
        field: 'files',
        filename: 'alpha.txt',
        contentType: 'text/plain',
        content: 'alpha batch text',
      },
      {
        field: 'files',
        filename: 'beta.md',
        contentType: 'text/markdown',
        content: '# beta\ncontent',
      },
    ]);

    const resp = await request.fetch(`${SPEC013_BACKEND}/api/v1/documents/upload/batch`, {
      method: 'POST',
      headers: {
        ...tenantHeaders(tenantId, workspaceId),
        'Content-Type': `multipart/form-data; boundary=${boundary}`,
      },
      data: body,
    });
    expect(resp.ok()).toBeTruthy();
    const json = await resp.json();
    expect(json.total_files).toBe(2);
    expect(json.failed).toBe(0);
    expect(Array.isArray(json.results)).toBeTruthy();
    expect(json.results).toHaveLength(2);
  });

  test('batch PDF upload accepts multiple PDFs in one request', async ({ request }) => {
    const { tenantId, workspaceId } = await createTenantWorkspaceViaApi(request, 'issue-236-pdf');
    const pdfFixture = fs.readFileSync(
      path.join(
        __dirname,
        '..',
        '..',
        'legacy',
        'edgequake-pdf',
        'test-data',
        '001_simple_text.pdf'
      )
    );
    const { boundary, body } = multipartBody([
      { field: 'files', filename: 'a.pdf', contentType: 'application/pdf', content: pdfFixture },
      { field: 'files', filename: 'b.pdf', contentType: 'application/pdf', content: pdfFixture },
    ]);

    const resp = await request.fetch(`${SPEC013_BACKEND}/api/v1/documents/pdf/batch`, {
      method: 'POST',
      headers: {
        ...tenantHeaders(tenantId, workspaceId),
        'Content-Type': `multipart/form-data; boundary=${boundary}`,
      },
      data: body,
    });
    expect(resp.ok()).toBeTruthy();
    const json = await resp.json();
    expect(json.total_files).toBe(2);
    expect(json.failed).toBe(0);
    expect(Array.isArray(json.results)).toBeTruthy();
    expect(json.results).toHaveLength(2);
    expect(
      json.results.some((r: { status: string }) =>
        ['processing', 'duplicate', 'reindexing'].includes(r.status)
      )
    ).toBeTruthy();
  });
});

