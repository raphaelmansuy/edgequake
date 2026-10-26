import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { expect, type APIRequestContext, type Page } from '@playwright/test';
export const API = process.env.SPEC180_API_URL || 'http://127.0.0.1:18480/api/v1';
export const TENANT = '00000000-0000-0000-0000-000000000002';
export const WS = '00000000-0000-0000-0000-000000000003';
export const DOC = '18000000-0000-4000-8000-000000000041';
export const HIDDEN = '18000000-0000-4000-8000-000000000042';
export const FINANCE = '18000000-0000-4000-8000-000000000021';
export const LEGAL = '18000000-0000-4000-8000-000000000022';
export const PASSWORD = 'W7!rQ9#vL2@pZ6'; // Disposable fixture credentials, never production.
export function fixturePdf(text:string):Buffer {
  const stream=`BT /F1 12 Tf 30 60 Td (${text.replace(/[()\\]/g,'')}) Tj ET`;
  const objects=['<< /Type /Catalog /Pages 2 0 R >>','<< /Type /Pages /Kids [3 0 R] /Count 1 >>','<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>','<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',`<< /Length ${Buffer.byteLength(stream)} >>\nstream\n${stream}\nendstream`];
  let pdf='%PDF-1.4\n';const offsets=[0];for(const [i,object]of objects.entries()){offsets.push(Buffer.byteLength(pdf));pdf+=`${i+1} 0 obj\n${object}\nendobj\n`;}
  const start=Buffer.byteLength(pdf);pdf+=`xref\n0 ${objects.length+1}\n0000000000 65535 f \n`+offsets.slice(1).map(offset=>`${String(offset).padStart(10,'0')} 00000 n \n`).join('')+`trailer\n<< /Size ${objects.length+1} /Root 1 0 R >>\nstartxref\n${start}\n%%EOF\n`;return Buffer.from(pdf);
}
export function sql(query: string): string {
  return execFileSync('docker', ['exec', '-i', 'edgequake-spec180-postgres', 'psql', '-U', 'edgequake', '-d', 'edgequake_spec180', '-v', 'ON_ERROR_STOP=1', '-Atc', query], { encoding: 'utf8' }).trim();
}
export function resetFixtures() {
  execFileSync('docker', ['exec', '-i', 'edgequake-spec180-postgres', 'psql', '-U', 'edgequake', '-d', 'edgequake_spec180'], { input: readFileSync(path.resolve('../specs/180-improve-sec-doc/e2e/seed.sql')), stdio: ['pipe', 'pipe', 'pipe'] });
  sql(`UPDATE public.documents SET created_at=clock_timestamp() WHERE id IN ('${DOC}','${HIDDEN}','18000000-0000-4000-8000-000000000043'); UPDATE public.documents SET classification_rank=2,compartment_ids=ARRAY['${FINANCE}'::uuid],label_version=1 WHERE id='${DOC}'; UPDATE public.documents SET classification_rank=3,compartment_ids=ARRAY['${LEGAL}'::uuid,'18000000-0000-4000-8000-000000000023'::uuid],label_version=1 WHERE id='${HIDDEN}'; UPDATE public.documents SET classification_rank=5,compartment_ids='{}',label_version=1 WHERE id='18000000-0000-4000-8000-000000000043'; DELETE FROM public.document_labels WHERE document_id='${DOC}'; INSERT INTO public.document_labels(document_id,tag_id) VALUES('${DOC}','${FINANCE}');`);
}
export async function login(request: APIRequestContext, persona = 'owner') {
  const response = await request.post(`${API}/auth/login`, { data: { username: `spec180${persona}`, password: PASSWORD } });
  expect(response.status(), await response.text()).toBe(200);
  const session = await response.json();
  return { Authorization: `Bearer ${session.access_token}`, 'X-Tenant-ID': TENANT, 'X-Workspace-ID': WS };
}
export async function readAccess(request: APIRequestContext, headers: Record<string,string>, id = DOC) {
  const response = await request.get(`${API}/documents/${id}/access`, { headers });
  expect(response.status(), await response.text()).toBe(200); return response.json();
}
export async function openLibrary(page: Page, persona = 'owner') {
  if(process.env.SPEC180_DEBUG==='1') {
    page.on('response', response => { if (/assignment-options|\/access$/.test(response.url())) console.log('access response', response.status(), response.url()); });
    page.on('requestfailed', request => console.log('failed request', request.url(), request.failure()?.errorText));
  }
  await page.addInitScript(({ tenant, workspace }) => {
    localStorage.setItem('tenantId', tenant); localStorage.setItem('workspaceId', workspace);
    localStorage.setItem('edgequake-tenant', JSON.stringify({ state: { selectedTenantId: tenant, selectedWorkspaceId: workspace }, version: 1 }));
  }, { tenant: TENANT, workspace: WS });
  await page.goto('/login');
  await page.locator('#username').fill(`spec180${persona}`);
  await page.locator('#password').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in/i }).click();
  await expect(page).not.toHaveURL(/\/login/, { timeout: 30_000 });
  await page.goto('/documents');
  await expect(page.getByText('Quarterly financial outlook', { exact: true }).first()).toBeVisible({ timeout: 40_000 });
}
export async function openEditor(page: Page) {
  await page.getByText('Quarterly financial outlook', { exact: true }).first().click();
  await page.getByTestId('access-property').click();
  const editor = page.getByTestId('document-access-editor'); await expect(editor).toBeVisible(); return editor;
}
export async function shot(page: Page, name: string) {
  const directory = path.resolve('../specs/180-improve-sec-doc/e2e/screenshots'); mkdirSync(directory, { recursive: true });
  await page.screenshot({ path: path.join(directory, `${name}.png`), fullPage: true, animations: 'disabled' });
}
