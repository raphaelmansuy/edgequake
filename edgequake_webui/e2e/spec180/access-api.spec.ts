import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { API, DOC, HIDDEN, FINANCE, LEGAL, WS, login, readAccess, resetFixtures, sql } from './fixtures';

test.describe('@spec180live SPEC-180 live authorization and transactions', () => {
  test.beforeEach(() => resetFixtures());
  test('assignment options intersect RBAC and ABAC; readonly steward cannot write', async ({ request }) => {
    const writer = await login(request, 'writer');
    const options = await (await request.get(`${API}/workspaces/${WS}/clearance/assignment-options`, { headers: writer })).json();
    expect(options.tags.map((tag: {id:string}) => tag.id)).toEqual([FINANCE]);
    expect(options.classifications).toEqual(['public','internal','confidential']);
    const readonly = await login(request, 'readonly');
    const access = await readAccess(request, readonly);
    expect(access.capabilities.can_edit).toBe(false);
    const response = await request.patch(`${API}/documents/${DOC}/labels`, { headers: { ...readonly, 'If-Match': `"label-${access.label_version}"`, 'Idempotency-Key': randomUUID() }, data: { ...access.access, classification: 'restricted' } });
    expect(response.status()).toBe(403);
  });
  test('hidden and missing access are indistinguishable; counts contain only visible documents', async ({ request }) => {
    const headers = await login(request, 'writer');
    for (const id of [HIDDEN, randomUUID()]) expect((await request.get(`${API}/documents/${id}/access`, { headers })).status()).toBe(404);
    const inventory = await (await request.get(`${API}/documents?classification=confidential&document_pattern=Atlas%20partnership%20agreement`, { headers })).json();
    expect(inventory.total).toBe(0); expect(inventory.documents).toEqual([]);
    expect(JSON.stringify(inventory)).not.toContain('Atlas partnership agreement');
  });
  test('conditional save, replay and descendant inheritance are atomic', async ({ request }) => {
    const headers = await login(request); const before = await readAccess(request, headers); const key = randomUUID();
    const data = { ...before.access, classification: 'confidential', compartment_ids: [FINANCE, LEGAL] };
    const saveHeaders = { ...headers, 'If-Match': `"label-${before.label_version}"`, 'Idempotency-Key': key };
    const saved = await request.patch(`${API}/documents/${DOC}/labels`, { headers: saveHeaders, data });
    expect(saved.status(), await saved.text()).toBe(204);
    expect(saved.headers().etag).toBe('"label-2"');
    expect((await request.patch(`${API}/documents/${DOC}/labels`, { headers: saveHeaders, data })).status()).toBe(204);
    const after = await readAccess(request, headers); expect(after.label_version).toBe(2); expect(after.access.compartment_ids).toEqual([FINANCE,LEGAL]);
    expect(sql(`SELECT classification_rank||':'||cardinality(compartment_ids) FROM public.chunks WHERE document_id='${DOC}'`)).toBe('3:2');
    const receipt = await (await request.get(`${API}/workspaces/${WS}/clearance/operations/${key}`, { headers })).json(); expect(receipt.status).toBe('committed');
    const reused = await request.patch(`${API}/documents/${DOC}/labels`, { headers: saveHeaders, data: {...data,classification:'restricted'} }); expect(reused.status()).toBe(409);
  });
  test('missing preconditions, stale versions and stale epochs cannot mutate', async ({ request }) => {
    const headers = await login(request); const before = await readAccess(request, headers); const data = {...before.access, classification:'confidential'};
    expect((await request.patch(`${API}/documents/${DOC}/labels`, { headers, data })).status()).toBe(428);
    expect((await request.patch(`${API}/documents/${DOC}/labels`, { headers:{...headers,'If-Match':'"label-0"','Idempotency-Key':randomUUID()}, data })).status()).toBe(412);
    expect((await request.patch(`${API}/documents/${DOC}/labels`, { headers:{...headers,'If-Match':'"label-1"','Idempotency-Key':randomUUID()}, data:{...data,expected_policy_epoch:before.policy_epoch-1} })).status()).toBe(409);
    expect((await readAccess(request, headers)).label_version).toBe(1);
  });
  test('mixed strengthening and tag removal still requires a steward and reason', async ({ request }) => {
    const writer = await login(request,'writer'); const before = await readAccess(request,writer);
    const data = {...before.access,classification:'confidential',compartment_ids:[]};
    expect((await request.patch(`${API}/documents/${DOC}/labels`, {headers:{...writer,'If-Match':'"label-1"','Idempotency-Key':randomUUID()},data})).status()).toBe(403);
    const owner = await login(request); const current = await readAccess(request,owner);
    expect((await request.patch(`${API}/documents/${DOC}/labels`, {headers:{...owner,'If-Match':'"label-1"','Idempotency-Key':randomUUID()},data:{...data,expected_policy_epoch:current.policy_epoch}})).status()).toBe(422);
    expect((await readAccess(request,owner)).label_version).toBe(1);
  });
  test('one stale bulk item rolls back the complete selection and its audit', async ({ request }) => {
    const headers = await login(request); const first = await readAccess(request,headers); const second = await readAccess(request,headers,HIDDEN); const operation = randomUUID();
    const response = await request.post(`${API}/documents/labels/batch`, { headers, data:{operation_id:operation,items:[{document_id:DOC,label_version:first.label_version,access:{...first.access,classification:'restricted'}},{document_id:HIDDEN,label_version:0,access:second.access}]} });
    expect(response.status()).toBe(412); expect((await readAccess(request,headers)).label_version).toBe(1);
    expect(sql(`SELECT count(*) FROM public.audit_logs WHERE metadata->>'operation_id'='${operation}'`)).toBe('0');
  });
  test('concurrent edits have one winner and one policy conflict', async ({ request }) => {
    const headers = await login(request); const before = await readAccess(request,headers);
    const responses = await Promise.all(['confidential','restricted'].map(classification => request.patch(`${API}/documents/${DOC}/labels`, {headers:{...headers,'If-Match':'"label-1"','Idempotency-Key':randomUUID()},data:{...before.access,classification}})));
    expect(responses.map(r=>r.status()).sort()).toEqual([204,409]); expect((await readAccess(request,headers)).label_version).toBe(2);
  });
  test('typed upload labels are present before processing; metadata cannot impersonate access', async ({ request }) => {
    const headers = await login(request); const before = await readAccess(request,headers);
    const rejected = await request.post(`${API}/documents`,{headers,data:{content:'Rejected '+randomUUID(),metadata:{classification_rank:1},async_processing:true}}); expect(rejected.status()).toBe(422);
    const title='SPEC180 protected upload '+randomUUID();
    const response = await request.post(`${API}/documents`,{headers,data:{title,content:'Document security admission '+randomUUID(),access:{...before.access,classification:'confidential'},async_processing:true}});
    expect([200,201,202],await response.text()).toContain(response.status()); const result=await response.json();
    const access=await readAccess(request,headers,result.document_id); expect(access.access.classification).toBe('confidential');expect(access.label_version).toBe(1);
    const epoch=await readAccess(request,headers);expect(epoch.policy_epoch).toBe(before.policy_epoch);
  });
});
