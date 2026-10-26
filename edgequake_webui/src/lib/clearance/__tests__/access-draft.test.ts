import { describe,expect,it } from 'vitest';
import { accessBroadens,accessPayload,appendUploadAccess,canonicalAccess,sameAccess } from '../access-draft';

describe('SPEC-180 canonical access intent',()=>{
  it('treats tag permutation and duplicates as the same label',()=>{
    expect(sameAccess({classification:'confidential',compartment_ids:['b','a','a']},{classification:'confidential',compartment_ids:['a','b']})).toBe(true);
    const input={classification:'confidential' as const,compartment_ids:['b','a']};canonicalAccess(input);
    expect(input.compartment_ids).toEqual(['b','a']);
  });
  it('identifies tag removal even when sensitivity increases',()=>{
    expect(accessBroadens({classification:'internal',compartment_ids:['hr','atlas']},{classification:'restricted',compartment_ids:['hr']})).toBe(true);
    expect(accessBroadens({classification:'internal',compartment_ids:['hr']},{classification:'restricted',compartment_ids:['hr','atlas']})).toBe(false);
    expect(accessBroadens({classification:'unverified',compartment_ids:[]},{classification:'internal',compartment_ids:[]})).toBe(true);
  });
  it('uses the identical canonical payload for JSON and multipart',()=>{
    const payload=accessPayload({classification:'restricted',compartment_ids:['b','a','b']},42);
    const form=new FormData();appendUploadAccess(form,payload);
    expect(JSON.parse(String(form.get('access')))).toEqual(payload);
    expect(payload).toEqual({classification:'restricted',compartment_ids:['a','b'],expected_policy_epoch:42});
    const absent=new FormData();appendUploadAccess(absent);expect(absent.has('access')).toBe(false);
  });
});
