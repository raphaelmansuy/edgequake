import { beforeEach,describe,expect,it,vi } from 'vitest';
const context=vi.hoisted(()=>({value:{tenantId:'tenant-a',workspaceId:'workspace-a',userId:'user-a'}}));
vi.mock('@/lib/api/client-context',()=>({getTenantContext:()=>context.value}));
import { createUploadScopeGuard } from '../upload-scope-guard';
describe('frozen upload scope',()=>{
  beforeEach(()=>{context.value={tenantId:'tenant-a',workspaceId:'workspace-a',userId:'user-a'};});
  it('permits retries in the same account and workspace',()=>{const guard=createUploadScopeGuard();expect(guard).not.toThrow();expect(guard).not.toThrow();});
  for(const field of ['tenantId','workspaceId','userId'] as const)it(`stops a queued or retried upload after ${field} changes`,()=>{const guard=createUploadScopeGuard();context.value={...context.value,[field]:'changed'};expect(guard).toThrow('workspace or account changed');});
});
