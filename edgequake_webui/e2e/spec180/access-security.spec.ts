import { createHash, randomUUID } from 'node:crypto';
import { expect,test } from '@playwright/test';
import { API,DOC,HIDDEN,FINANCE,LEGAL,WS,fixturePdf,login,readAccess,resetFixtures,sql } from './fixtures';
test.describe('@spec180live SPEC-180 live failure and transport boundaries',()=>{
  test.beforeEach(()=>resetFixtures());
  test('PDF admission protects the staged original and derives its final label',async({request})=>{
    const headers=await login(request);const saved=await readAccess(request,headers);const data=fixturePdf('SPEC180 '+randomUUID());
    const response=await request.post(`${API}/documents/pdf`,{headers,multipart:{file:{name:'Security review.pdf',mimeType:'application/pdf',buffer:data},access:JSON.stringify({...saved.access,classification:'confidential',compartment_ids:[LEGAL]}),pdf_parser_backend:'edgeparse',enable_vision:'false'}});
    expect(response.status(),await response.text()).toBe(200);const admitted=await response.json();expect(admitted.document_id).toBeTruthy();const access=await readAccess(request,headers,admitted.document_id);expect(access.access.classification).toBe('confidential');
    expect(sql(`SELECT classification_rank||':'||cardinality(compartment_ids) FROM public.pdf_documents WHERE document_id='${admitted.document_id}'`)).toBe('3:1');
    const authorized=await request.get(`${API}/documents/pdf/${admitted.pdf_id}/download`,{headers:{...headers,Range:'bytes=0-15'}});expect(authorized.status(),await authorized.text()).toBe(206);expect((await authorized.body()).subarray(0,5).toString()).toBe('%PDF-');expect(authorized.headers()['cache-control']).toContain('no-store');
    expect((await request.get(`${API}/documents/pdf/${admitted.pdf_id}`,{headers})).status()).toBe(200);
    const writer=await login(request,'writer');expect((await request.get(`${API}/documents/${admitted.document_id}/original`,{headers:writer})).status()).toBe(404);
    expect((await request.get(`${API}/documents/pdf/progress/${admitted.task_id}`,{headers:writer})).status()).toBe(404);
    expect((await request.get(`${API}/tasks/${admitted.task_id}`,{headers})).status()).toBe(200);expect((await request.get(`${API}/tasks/${admitted.task_id}`,{headers:writer})).status()).toBe(404);
    const tasks=await request.get(`${API}/tasks?page_size=100`,{headers:writer});expect(tasks.status(),await tasks.text()).toBe(200);expect(JSON.stringify(await tasks.json())).not.toContain(admitted.task_id);
    for(const suffix of ['', '/content', '/download']) expect((await request.get(`${API}/documents/pdf/${admitted.pdf_id}${suffix}`,{headers:writer})).status()).toBe(404);
    const listed=await request.get(`${API}/documents/pdf?page_size=100`,{headers:writer});expect(listed.status()).toBe(200);expect(JSON.stringify(await listed.json())).not.toContain(admitted.pdf_id);
  });
  test('PDF duplicate intent cannot silently relabel the existing document',async({request})=>{
    const headers=await login(request);const saved=await readAccess(request,headers);const data=fixturePdf('Duplicate '+randomUUID());
    const upload=(access:unknown)=>request.post(`${API}/documents/pdf`,{headers,multipart:{file:{name:'Duplicate.pdf',mimeType:'application/pdf',buffer:data},access:JSON.stringify(access),pdf_parser_backend:'edgeparse',enable_vision:'false'}});
    const first=await upload({...saved.access,classification:'confidential'});expect(first.status(),await first.text()).toBe(200);const admitted=await first.json();
    const duplicate=await upload({...saved.access,classification:'internal'});expect(duplicate.status()).toBe(409);expect((await readAccess(request,headers,admitted.document_id)).access.classification).toBe('confidential');
  });
  test('MCP upload intent survives begin, write, commit and replay; revoked policy blocks commit',async({request})=>{
    const headers=await login(request);const saved=await readAccess(request,headers);
    const keyResponse=await request.post(`${API}/api-keys`,{headers,data:{name:'SPEC180 live MCP',scopes:['edgequake:read','edgequake:write'],expires_in_days:1}});expect(keyResponse.status()).toBe(201);const key=await keyResponse.json();
    const call=async(name:string,args:Record<string,unknown>,apiKey=key.api_key)=>{
      const response=await request.post(API.replace('/api/v1','/mcp'),{headers:{'X-API-Key':apiKey,'X-Tenant-ID':headers['X-Tenant-ID'],'X-Workspace-ID':WS,Accept:'application/json,text/event-stream','MCP-Protocol-Version':'2025-11-25','Mcp-Method':'tools/call','Mcp-Name':name},data:{jsonrpc:'2.0',id:randomUUID(),method:'tools/call',params:{name,arguments:{...args,workspace_id:WS}}}});
      expect(response.status(),await response.text()).toBe(200);const result=await response.json();return result.result?.structuredContent ?? result;
    };
    const bytes=Buffer.from('MCP security '+randomUUID());const access={...saved.access,classification:'confidential',compartment_ids:[LEGAL]};
    const begin=await call('eq_upload_begin',{filename:'MCP protected.txt',media_type:'text/plain',byte_length:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex'),chunked:true,access});expect(begin.ok,JSON.stringify(begin)).toBe(true);
    const write=await call('eq_upload_write',{upload_id:begin.upload_id,offset:0,chunk_base64:bytes.toString('base64')});expect(write.ok,JSON.stringify(write)).toBe(true);
    const commit=await call('eq_upload_commit',{upload_id:begin.upload_id});expect(commit.ok,JSON.stringify(commit)).toBe(true);expect((await readAccess(request,headers,commit.document_id)).access.classification).toBe('confidential');
    const replay=await call('eq_upload_commit',{upload_id:begin.upload_id});expect(replay.document_id).toBe(commit.document_id);
    const writer=await login(request,'writer');const writerKeyResponse=await request.post(`${API}/api-keys`,{headers:writer,data:{name:'SPEC180 reader MCP',scopes:['edgequake:read'],expires_in_days:1}});expect(writerKeyResponse.status()).toBe(201);const writerKey=await writerKeyResponse.json();
    const hidden=await call('eq_document_get',{document_id:commit.document_id,include:['metadata','text']},writerKey.api_key);expect(hidden.ok,JSON.stringify(hidden)).toBe(false);expect(JSON.stringify(hidden)).not.toContain('MCP_protected.txt');
    const hiddenTask=await call('eq_task_get',{task_id:commit.task_id},writerKey.api_key);expect(hiddenTask.ok,JSON.stringify(hiddenTask)).toBe(false);
    const listing=await call('eq_document_list',{},writerKey.api_key);expect(listing.ok).toBe(true);expect(JSON.stringify(listing)).not.toContain(commit.document_id);expect(JSON.stringify(listing)).not.toContain('Atlas partnership agreement');
    const stats=await call('eq_workspace_stats',{},writerKey.api_key);expect(stats.stats.document_count).toBe(listing.total);expect(stats.stats.count_scope).toBe('visible_documents');

    const stale=await call('eq_upload_begin',{filename:'Revoked.txt',media_type:'text/plain',chunked:true,access});expect(stale.ok).toBe(true);
    await call('eq_upload_write',{upload_id:stale.upload_id,offset:0,chunk_base64:Buffer.from('revoked').toString('base64')});
    sql(`UPDATE public.workspace_clearance_settings SET policy_epoch=policy_epoch+1 WHERE workspace_id='${WS}'`);
    const revoked=await call('eq_upload_commit',{upload_id:stale.upload_id});expect(revoked.ok,JSON.stringify(revoked)).toBe(false);expect(revoked.document_id).toBeUndefined();
  });
  test('the actual application database role cannot bypass clearance',()=>{
    const result=sql(`BEGIN; SET LOCAL ROLE edgequake_tenant_access; SELECT public.set_tenant_context('00000000-0000-0000-0000-000000000002','${WS}','18000000-0000-4000-8000-000000000012'); SELECT public.set_clearance_context('on','3','{${FINANCE}}','off','off','off'); SELECT current_user||'|'||count(*) FILTER(WHERE id='${DOC}')||'|'||count(*) FILTER(WHERE id='${HIDDEN}') FROM public.documents WHERE id IN ('${DOC}','${HIDDEN}'); COMMIT;`);
    expect(result).toContain('edgequake_tenant_access|1|0');
  });
  test('hidden documents are absent across content, original, chunks and status routes',async({request})=>{
    const headers=await login(request,'writer');
    for(const suffix of ['','/content','/original','/chunks','/status','/lineage']) {
      const response=await request.get(`${API}/documents/${HIDDEN}${suffix}`,{headers});expect(response.status()).toBe(404);expect(await response.text()).not.toContain('Atlas partnership agreement');
    }
  });
  test('revoked membership and readonly roles cannot admit or mutate documents',async({request})=>{
    const readonly=await login(request,'readonly');
    expect((await request.post(`${API}/documents`,{headers:readonly,data:{title:'Forbidden',content:randomUUID(),async_processing:true}})).status()).toBe(403);
    const writer=await login(request,'writer');const saved=await readAccess(request,writer);
    sql(`UPDATE public.memberships SET is_active=false WHERE workspace_id='${WS}' AND user_id='18000000-0000-4000-8000-000000000012'`);
    try {
      const denied=await request.patch(`${API}/documents/${DOC}/labels`,{headers:{...writer,'If-Match':'"label-1"','Idempotency-Key':randomUUID()},data:{...saved.access,classification:'confidential'}});expect([403,404]).toContain(denied.status());
      expect(sql(`SELECT label_version FROM public.documents WHERE id='${DOC}'`)).toBe('1');
    } finally {sql(`UPDATE public.memberships SET is_active=true WHERE workspace_id='${WS}' AND user_id='18000000-0000-4000-8000-000000000012'`);}
  });
  test('late worker updates derive current parent labels and profile tags cannot be deleted',()=>{
    sql(`UPDATE public.documents SET classification_rank=4 WHERE id='${DOC}'; UPDATE public.chunks SET classification_rank=1,compartment_ids='{}',uploaded_by=NULL WHERE document_id='${DOC}'`);
    expect(sql(`SELECT classification_rank||':'||cardinality(compartment_ids)||':'||uploaded_by FROM public.chunks WHERE document_id='${DOC}'`)).toBe('4:1:18000000-0000-4000-8000-000000000011');
    expect(()=>sql(`DELETE FROM public.clearance_tags WHERE tag_id='${FINANCE}'`)).toThrow();
    expect(sql(`SELECT count(*) FROM pg_trigger WHERE tgname='z_document_child_access' AND NOT tgisinternal AND tgparentid=0`)).toBe('7');
  });
  test('operation receipts remain private to the actor and duplicate selections do not write',async({request})=>{
    const owner=await login(request);const saved=await readAccess(request,owner);const operation=randomUUID();
    const result=await request.patch(`${API}/documents/${DOC}/labels`,{headers:{...owner,'If-Match':'"label-1"','Idempotency-Key':operation},data:{...saved.access,classification:'confidential'}});expect(result.status()).toBe(204);
    const writer=await login(request,'writer');const status=await request.get(`${API}/workspaces/${WS}/clearance/operations/${operation}`,{headers:writer});expect((await status.json()).status).toBe('unknown');
    const after=await readAccess(request,owner);const item={document_id:DOC,label_version:2,access:after.access};
    const duplicate=await request.post(`${API}/documents/labels/batch`,{headers:owner,data:{operation_id:randomUUID(),items:[item,item]}});expect(duplicate.status()).toBe(400);expect((await readAccess(request,owner)).label_version).toBe(2);
  });
  test('audit failure rolls back label, descendants, epoch and receipt',async({request})=>{
    const headers=await login(request);const before=await readAccess(request,headers);const key=randomUUID();
    sql(`CREATE OR REPLACE FUNCTION public.spec180_reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.resource_id='${DOC}' AND NEW.metadata->>'source'='document_access' THEN RAISE EXCEPTION 'SPEC180 injected audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER spec180_reject_audit BEFORE INSERT ON public.audit_logs FOR EACH ROW EXECUTE FUNCTION public.spec180_reject_audit();`);
    try {
      const result=await request.patch(`${API}/documents/${DOC}/labels`,{headers:{...headers,'If-Match':'"label-1"','Idempotency-Key':key},data:{...before.access,classification:'restricted'}});expect(result.status()).toBe(500);
      const after=await readAccess(request,headers);expect(after.label_version).toBe(before.label_version);expect(after.policy_epoch).toBe(before.policy_epoch);
      expect(sql(`SELECT classification_rank FROM public.chunks WHERE document_id='${DOC}'`)).toBe('2');expect(sql(`SELECT count(*) FROM public.document_access_operations WHERE operation_id='${key}'`)).toBe('0');
    } finally {sql('DROP TRIGGER spec180_reject_audit ON public.audit_logs; DROP FUNCTION public.spec180_reject_audit();');}
  });
  test('server bulk add preserves sensitivity and remove requires an audited reason',async({request})=>{
    const headers=await login(request);const before=await readAccess(request,headers);
    const data={operation:'add_tags',items:[{document_id:DOC,label_version:1}],compartment_ids:[LEGAL],expected_policy_epoch:before.policy_epoch,operation_id:randomUUID()};
    const add=await request.post(`${API}/documents/labels/batch`,{headers,data});expect(add.status(),await add.text()).toBe(200);
    const replay=await request.post(`${API}/documents/labels/batch`,{headers,data});expect(replay.status(),await replay.text()).toBe(200);expect((await readAccess(request,headers)).label_version).toBe(2);
    const after=await readAccess(request,headers);expect(after.access.classification).toBe('internal');expect(after.access.compartment_ids).toEqual([FINANCE,LEGAL]);
    const remove=await request.post(`${API}/documents/labels/batch`,{headers,data:{operation:'remove_tags',items:[{document_id:DOC,label_version:2}],compartment_ids:[LEGAL],expected_policy_epoch:after.policy_epoch,reason:'Approved for finance readers following legal review.',operation_id:randomUUID()}});expect(remove.status(),await remove.text()).toBe(200);expect((await readAccess(request,headers)).access.compartment_ids).toEqual([FINANCE]);
  });
  test('member lookup and exact draft check are steward-only',async({request})=>{
    const owner=await login(request);const before=await readAccess(request,owner);
    const members=await request.get(`${API}/workspaces/${WS}/clearance/members?q=writer`,{headers:owner});expect(members.status()).toBe(200);expect((await members.json()).members).toContainEqual({user_id:'18000000-0000-4000-8000-000000000012',name:'spec180writer'});
    const payload={mode:'edit',items:[{document_id:DOC,label_version:1,access:before.access}],expected_policy_epoch:before.policy_epoch,check_member_id:'18000000-0000-4000-8000-000000000012'};
    const checked=await request.post(`${API}/workspaces/${WS}/clearance/preview`,{headers:owner,data:payload});expect(checked.status()).toBe(200);expect((await checked.json()).items[0].member_can_read).toBe(true);
    const writer=await login(request,'writer');expect((await request.get(`${API}/workspaces/${WS}/clearance/members?q=owner`,{headers:writer})).status()).toBe(403);expect((await request.post(`${API}/workspaces/${WS}/clearance/preview`,{headers:writer,data:payload})).status()).toBe(403);
  });
  test('opaque tag cursors are invalidated by a policy change',async({request})=>{
    sql(`INSERT INTO public.clearance_tags(workspace_id,kind,key,value) SELECT '${WS}','compartment','spec180-pagination','ZZ pagination '||n FROM generate_series(1,55) n ON CONFLICT DO NOTHING`);
    try {
    const headers=await login(request);const first=await request.get(`${API}/workspaces/${WS}/clearance/assignment-options`,{headers});expect(first.status()).toBe(200);const page=await first.json();expect(page.tags).toHaveLength(50);expect(page.next_cursor).toBeTruthy();
    sql(`UPDATE public.workspace_clearance_settings SET policy_epoch=policy_epoch+1 WHERE workspace_id='${WS}'`);
    const stale=await request.get(`${API}/workspaces/${WS}/clearance/assignment-options?cursor=${encodeURIComponent(page.next_cursor)}`,{headers});expect(stale.status()).toBe(409);expect((await stale.json()).code).toBe('POLICY_CHANGED');
    } finally {sql(`DELETE FROM public.clearance_tags WHERE workspace_id='${WS}' AND key='spec180-pagination'`);}
  });
  for(const malformed of [null,{}, {classification:'public'}, {classification:'unrecognized',compartment_ids:[],expected_policy_epoch:0}])test(`malformed JSON access is rejected: ${JSON.stringify(malformed)}`,async({request})=>{
    const headers=await login(request);const response=await request.post(`${API}/documents`,{headers,data:{title:'Rejected envelope',content:randomUUID(),async_processing:true,access:malformed}});expect([400,409,422]).toContain(response.status());
  });
  test('multipart field order has the same initial access as JSON',async({request})=>{
    const headers=await login(request);const before=await readAccess(request,headers);const access={...before.access,classification:'confidential'};
    for(const accessFirst of [true,false]) {
      const boundary='spec180-'+randomUUID();const bodyParts=[`Content-Disposition: form-data; name="access"\r\n\r\n${JSON.stringify(access)}`,`Content-Disposition: form-data; name="file"; filename="security-${randomUUID()}.txt"\r\nContent-Type: text/plain\r\n\r\nSPEC180 multipart protected content ${randomUUID()}`];if(!accessFirst)bodyParts.reverse();
      const data=Buffer.from(bodyParts.map(part=>`--${boundary}\r\n${part}\r\n`).join('')+`--${boundary}--\r\n`);
      const response=await request.post(`${API}/documents/upload`,{headers:{...headers,'Content-Type':`multipart/form-data; boundary=${boundary}`},data});expect([200,201,202],await response.text()).toContain(response.status());const result=await response.json();expect((await readAccess(request,headers,result.document_id)).access.classification).toBe('confidential');
    }
  });
  test('omitted access remains private review and a different dedup intent conflicts',async({request})=>{
    const headers=await login(request);const content='SPEC180 immutable dedup '+randomUUID();const body={title:'Needs review upload',content,async_processing:true};
    const first=await request.post(`${API}/documents`,{headers,data:body});expect([200,201,202],await first.text()).toContain(first.status());const result=await first.json();const access=await readAccess(request,headers,result.document_id);expect(access.access.classification).toBe('unverified');
    const writer=await login(request,'writer');expect((await request.get(`${API}/documents/${result.document_id}/access`,{headers:writer})).status()).toBe(404);
    const duplicate=await request.post(`${API}/documents`,{headers,data:{...body,access:{classification:'internal',compartment_ids:[FINANCE],expected_policy_epoch:access.policy_epoch}}});expect(duplicate.status()).toBe(409);expect((await readAccess(request,headers,result.document_id)).access.classification).toBe('unverified');
  });
  test('invalid and out-of-scope tag IDs fail without disclosing vocabulary',async({request})=>{
    const headers=await login(request,'writer');const saved=await readAccess(request,headers);
    for(const id of [LEGAL,randomUUID()]) {
      const result=await request.patch(`${API}/documents/${DOC}/labels`,{headers:{...headers,'If-Match':'"label-1"','Idempotency-Key':randomUUID()},data:{...saved.access,classification:'confidential',compartment_ids:[id]}});expect([403,422]).toContain(result.status());expect(await result.text()).not.toContain(id);
    }
    expect((await readAccess(request,headers)).label_version).toBe(1);
  });
});
