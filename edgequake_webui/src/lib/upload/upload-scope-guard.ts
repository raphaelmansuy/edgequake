import { getTenantContext } from '@/lib/api/client-context';

/** Freeze a batch's account/workspace intent, including retries after refresh. */
export function createUploadScopeGuard(): () => void {
  const expected=JSON.stringify(getTenantContext());
  return () => {
    if (JSON.stringify(getTenantContext())!==expected) throw new Error('Upload stopped because your workspace or account changed. Review the files again.');
  };
}
