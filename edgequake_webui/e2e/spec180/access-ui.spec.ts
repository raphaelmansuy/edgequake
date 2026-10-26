import { expect, test } from '@playwright/test';
import { openLibrary, openEditor, resetFixtures, shot, API, DOC, FINANCE, LEGAL, WS, login, readAccess, sql } from './fixtures';
import { randomUUID } from 'node:crypto';
import AxeBuilder from '@axe-core/playwright';
test.describe('@spec180live SPEC-180 live document security UX',()=>{
  test.beforeEach(()=>resetFixtures());
  test('explicit property save, cancel, keyboard discard and saved state survive reload',async({page,request})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);const editor=await openEditor(page);
    await shot(page,'desktop-property-light');
    await editor.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();
    await page.keyboard.press('Escape');await expect(editor.getByText('Discard your unsaved access changes?')).toBeVisible();
    await editor.getByRole('button',{name:'Keep editing'}).click();await editor.getByRole('button',{name:'Cancel',exact:true}).click();
    const headers=await login(request);expect((await readAccess(request,headers)).access.classification).toBe('internal');
    await page.getByTestId('access-property').click();const next=page.getByTestId('document-access-editor');
    await next.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();
    await expect(next.getByRole('button',{name:'Save access',exact:true})).toBeEnabled();await shot(page,'desktop-property-draft');
    await next.getByRole('button',{name:'Save access',exact:true}).click();await expect(next).toHaveCount(0);
    await page.reload();expect((await readAccess(request,headers)).access.classification).toBe('confidential');
  });
  test('upload review keeps files local, permits governed tags and per-file overrides',async({page})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);
    let admissions=0;page.on('request',req=>{if(req.method()==='POST'&&/\/documents(\/upload|\/file|\/pdf)?$/.test(new URL(req.url()).pathname))admissions++;});
    await page.locator('input[type=file]').first().setInputFiles([{name:'financial-outlook.txt',mimeType:'text/plain',buffer:Buffer.from('Financial outlook')},{name:'partnership-notes.txt',mimeType:'text/plain',buffer:Buffer.from('Partnership notes')}]);
    const review=page.getByTestId('upload-access-review');await expect(review).toBeVisible();expect(admissions).toBe(0);
    await review.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Internal',exact:true}).click();
    await review.getByTestId('security-tag-picker-trigger').click();await page.getByRole('checkbox',{name:'Finance',exact:true}).click();
    await shot(page,'desktop-upload-tag-picker');await page.getByRole('button',{name:'Done',exact:true}).click();
    await review.getByRole('button',{name:'Customize',exact:true}).last().click();await review.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();
    await expect(review.getByRole('button',{name:'Upload 2 files',exact:true})).toBeEnabled();await shot(page,'desktop-upload-override');expect(admissions).toBe(0);await review.getByRole('button',{name:'Cancel',exact:true}).click();expect(admissions).toBe(0);
  });
  for(const width of [768,375])test(`responsive access editor ${width}px and dark mode`,async({page})=>{
    await page.setViewportSize({width,height:900});await openLibrary(page);const editor=await openEditor(page);
    await expect(editor).toBeInViewport();await shot(page,`${width}-property-light`);
    await page.evaluate(()=>document.documentElement.classList.add('dark'));await shot(page,`${width}-property-dark`);
    const box=await editor.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width);
    await editor.getByTestId('security-tag-picker-trigger').click();await expect(page.getByRole('checkbox',{name:'Legal',exact:true})).toBeVisible();await shot(page,`${width}-tag-picker-dark`);
  });
  for(const width of [768,375])test(`responsive upload review ${width}px stays local and fits in both themes`,async({page})=>{
    await page.setViewportSize({width,height:900});await openLibrary(page);
    let admissions=0;page.on('request',request=>{if(request.method()==='POST'&&/\/documents(\/upload|\/pdf)?$/.test(new URL(request.url()).pathname))admissions++;});
    await page.locator('input[type=file]').first().setInputFiles([{name:'Quarterly report.txt',mimeType:'text/plain',buffer:Buffer.from('Report')},{name:'Partnership notes.txt',mimeType:'text/plain',buffer:Buffer.from('Notes')}]);
    const review=page.getByTestId('upload-access-review');await expect(review).toBeVisible();
    await review.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Internal',exact:true}).click();
    await review.getByTestId('security-tag-picker-trigger').click();await page.getByRole('checkbox',{name:'Finance',exact:true}).click();await page.getByRole('button',{name:'Done',exact:true}).click();
    const upload=review.getByRole('button',{name:'Upload 2 files',exact:true});await expect(upload).toBeEnabled();await expect(upload).toBeInViewport();
    const box=await review.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width);expect(box!.y+box!.height).toBeLessThanOrEqual(900);
    await shot(page,`${width}-upload-light`);await page.evaluate(()=>document.documentElement.classList.add('dark'));await shot(page,`${width}-upload-dark`);
    expect(admissions).toBe(0);await review.getByRole('button',{name:'Cancel',exact:true}).click();expect(admissions).toBe(0);
  });
  test('readonly steward sees a truthful disabled editor and hidden document is absent for writer',async({page})=>{
    await openLibrary(page,'readonly');const editor=await openEditor(page);await expect(editor.getByRole('button',{name:'Save access',exact:true})).toBeDisabled();await expect(editor.getByText('Your role cannot change document access.')).toBeVisible();await shot(page,'desktop-property-readonly');
  });
  test('upload button admits a file with the reviewed label and no relabel PATCH',async({page,request})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);const name=`Board outlook ${randomUUID().slice(0,8)}.txt`;let patches=0;page.on('request',r=>{if(r.method()==='PATCH'&&r.url().endsWith('/labels'))patches++;});
    await page.locator('input[type=file]').first().setInputFiles({name,mimeType:'text/plain',buffer:Buffer.from('SPEC180 live UI upload '+randomUUID())});const review=page.getByTestId('upload-access-review');await expect(review).toBeVisible();
    await review.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();await review.getByTestId('security-tag-picker-trigger').click();await page.getByRole('checkbox',{name:'Finance',exact:true}).click();await page.getByRole('button',{name:'Done',exact:true}).click();
    await expect(review.getByRole('button',{name:'Upload 1 file',exact:true})).toBeEnabled();await shot(page,'desktop-upload-ready');
    const admitted=page.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname==='/api/v1/documents');await review.getByRole('button',{name:'Upload 1 file',exact:true}).click();const response=await admitted;expect([200,201,202],await response.text()).toContain(response.status());const result=await response.json();await expect(review).toHaveCount(0);
    const headers=await login(request);const saved=await readAccess(request,headers,result.document_id);expect(saved.access.classification).toBe('confidential');expect(saved.access.compartment_ids).toEqual([FINANCE]);expect(patches).toBe(0);
  });
  test('default review upload is usable and remains hidden to another ordinary member',async({page,request})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);
    await page.locator('input[type=file]').first().setInputFiles({name:'Private review notes.txt',mimeType:'text/plain',buffer:Buffer.from('Private review '+randomUUID())});
    const review=page.getByTestId('upload-access-review');await expect(review).toBeVisible();await expect(review.getByRole('combobox',{name:'Sensitivity'})).toHaveText('Needs review');
    const upload=review.getByRole('button',{name:'Upload 1 file',exact:true});await expect(upload).toBeEnabled();await shot(page,'desktop-upload-needs-review');
    const admitted=page.waitForResponse(response=>response.request().method()==='POST'&&new URL(response.url()).pathname==='/api/v1/documents');await upload.click();const response=await admitted;expect([200,201,202],await response.text()).toContain(response.status());const result=await response.json();
    const saved=await readAccess(request,await login(request),result.document_id);expect(saved.access.classification).toBe('unverified');
    expect((await request.get(`${API}/documents/${result.document_id}/content`,{headers:await login(request,'writer')})).status()).toBe(404);
  });
  test('rules-off uploads retain the legacy direct-admission contract',async({page})=>{
    sql(`UPDATE public.workspace_clearance_settings SET enforcement=false,policy_epoch=policy_epoch+1 WHERE workspace_id='${WS}'`);
    try {
      await openLibrary(page);await expect(page.getByText('Document access rules are off for this workspace.',{exact:true}).first()).toBeVisible();
      const admitted=page.waitForResponse(response=>response.request().method()==='POST'&&new URL(response.url()).pathname==='/api/v1/documents');
      await page.locator('input[type=file]').first().setInputFiles({name:'Legacy direct upload.txt',mimeType:'text/plain',buffer:Buffer.from('Legacy direct '+randomUUID())});
      const response=await admitted;expect([200,201,202],await response.text()).toContain(response.status());expect(response.request().postDataJSON().access).toBeUndefined();await expect(page.getByTestId('upload-access-review')).toHaveCount(0);
      const result=await response.json();expect(sql(`SELECT label_version FROM public.documents WHERE id='${result.document_id}'`)).toBe('0');
    } finally {sql(`UPDATE public.workspace_clearance_settings SET enforcement=true,policy_epoch=policy_epoch+1 WHERE workspace_id='${WS}'`);}
  });
  test('bulk add preserves each sensitivity and changes only affected documents',async({page,request})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);
    for(const title of ['Quarterly financial outlook','Atlas partnership agreement'])await page.getByRole('row').filter({has:page.getByText(title,{exact:true})}).getByRole('checkbox',{name:'Select',exact:true}).click();
    await page.getByTestId('bulk-access-trigger').click();const editor=page.getByTestId('bulk-access-editor');await editor.getByTestId('security-tag-picker-trigger').click();await page.getByRole('checkbox',{name:'Legal',exact:true}).click();await page.getByRole('button',{name:'Done',exact:true}).click();await expect(editor.getByText('1 of 2 documents will change.')).toBeVisible();await shot(page,'desktop-bulk-add');await editor.getByRole('button',{name:'Apply to 2 documents'}).click();await expect(editor).toHaveCount(0);
    const headers=await login(request);const saved=await readAccess(request,headers);expect(saved.access.classification).toBe('internal');expect(saved.access.compartment_ids).toEqual([FINANCE,LEGAL]);
  });
  test('stale policy retains a draft and requires reviewed retry',async({page,request})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);const editor=await openEditor(page);await editor.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();await expect(editor.getByRole('button',{name:'Save access',exact:true})).toBeEnabled();
    sql(`UPDATE public.workspace_clearance_settings SET policy_epoch=policy_epoch+1 WHERE workspace_id='${WS}'`);await editor.getByRole('button',{name:'Save access',exact:true}).click();await expect(editor.getByRole('alert').first()).toBeVisible();await expect(editor.getByRole('combobox',{name:'Sensitivity'})).toHaveText('Confidential');await shot(page,'desktop-property-conflict');
    await editor.getByRole('button',{name:'Check status / reload access'}).click();await expect(editor.getByRole('combobox',{name:'Sensitivity'})).toHaveText('Confidential');await expect(editor.getByRole('button',{name:'Save access',exact:true})).toBeEnabled();await editor.getByRole('button',{name:'Save access',exact:true}).click();await expect(editor).toHaveCount(0);expect((await readAccess(request,await login(request))).access.classification).toBe('confidential');
  });
  test('lost save response reconciles a committed operation without duplicate audit',async({page,request})=>{
    await openLibrary(page);const editor=await openEditor(page);await editor.getByRole('combobox',{name:'Sensitivity'}).click();await page.getByRole('option',{name:'Confidential',exact:true}).click();await expect(editor.getByRole('button',{name:'Save access',exact:true})).toBeEnabled();
    await page.route(`**/documents/${DOC}/labels`,async route=>{await route.fetch();await route.abort('connectionreset');},{times:1});await editor.getByRole('button',{name:'Save access',exact:true}).click();await expect(editor.getByRole('button',{name:'Check status / reload access'})).toBeVisible();await shot(page,'desktop-property-unknown-outcome');await editor.getByRole('button',{name:'Check status / reload access'}).click();await expect(editor).toHaveCount(0);expect((await readAccess(request,await login(request))).label_version).toBe(2);
  });
  test('member check evaluates the draft and keyboard picker keeps its parent open',async({page})=>{
    await page.setViewportSize({width:1440,height:1000});await openLibrary(page);const editor=await openEditor(page);await editor.getByText('Check a workspace member',{exact:true}).click();await editor.getByLabel('Find a member').fill('writer');await editor.getByRole('button',{name:'spec180writer',exact:true}).click();await expect(editor.getByText('Can read under this draft.',{exact:true})).toBeVisible();
    await editor.getByTestId('security-tag-picker-trigger').click();await page.getByRole('checkbox',{name:'Legal',exact:true}).click();await page.keyboard.press('Escape');await expect(editor).toBeVisible();await expect(editor.getByText('Cannot read under this draft.',{exact:true})).toBeVisible();await shot(page,'desktop-member-draft-check');await page.keyboard.press('Escape');await expect(editor.getByText('Discard your unsaved access changes?')).toBeVisible();
  });
  test('access dialog passes focused WCAG checks',async({page})=>{
    await openLibrary(page);const editor=await openEditor(page);await editor.evaluate(async element=>{await Promise.all(element.getAnimations().map(animation=>animation.finished));});const result=await new AxeBuilder({page}).include('[data-testid="document-access-editor"]').withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();expect(result.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))).toEqual([]);
  });
});
