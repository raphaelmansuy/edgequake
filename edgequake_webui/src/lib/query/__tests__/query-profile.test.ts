import { describe, expect, it } from 'vitest';
import { GRAPH_CAPABILITIES } from '@/constants/rag-profile';
import { capabilitiesForDraft } from '@/constants/rag-profile';
import type { QuerySettings } from '@/types/settings';
import { buildChatRequest, buildDocumentFilter } from '../build-chat-request';
import { applyProfile, effectiveMode, modesForProfile } from '../query-profile';
import { chatRequestBody } from '@/lib/api/chat';

const normal = capabilitiesForDraft('normal', 'text');
const settings: QuerySettings = {
  mode: 'mix',
  topK: 10,
  maxTokens: 2048,
  temperature: 0.7,
  stream: true,
};

describe('modesForProfile', () => {
  it('a graph workspace lists every mode', () => {
    expect(modesForProfile(GRAPH_CAPABILITIES).map((m) => m.id)).toEqual([
      'local', 'global', 'hybrid', 'mix', 'naive', 'bypass',
    ]);
  });

  it('a normal workspace lists chunks and chat only', () => {
    expect(modesForProfile(normal).map((m) => m.id)).toEqual(['naive', 'bypass']);
  });
});

describe('effectiveMode', () => {
  it('keeps a runnable mode and replaces a hidden one with chunks', () => {
    expect(effectiveMode('bypass', normal)).toBe('bypass');
    expect(effectiveMode('naive', normal)).toBe('naive');
    expect(effectiveMode('mix', normal)).toBe('naive');
    expect(effectiveMode('local', normal)).toBe('naive');
    expect(effectiveMode('mix', GRAPH_CAPABILITIES)).toBe('mix');
  });
});

describe('applyProfile', () => {
  it('returns the same object when nothing changes (stable identity)', () => {
    expect(applyProfile(settings, GRAPH_CAPABILITIES, [])).toBe(settings);
  });

  it('shows a normal workspace chunks and its kind chips, without touching the saved mode', () => {
    const out = applyProfile(settings, normal, ['image']);
    expect(out.mode).toBe('naive');
    expect(out.docKinds).toEqual(['image']);
    expect(settings.mode).toBe('mix');
  });

  it('never carries kind chips onto a graph workspace', () => {
    expect(applyProfile(settings, GRAPH_CAPABILITIES, ['image']).docKinds).toBeUndefined();
  });
});

describe('request building', () => {
  it('writes the chips into document_filter.doc_kinds', () => {
    expect(buildDocumentFilter({ docKinds: ['pdf', 'image'] })).toEqual({
      document_ids: undefined,
      doc_kinds: ['pdf', 'image'],
    });
  });

  it('sends no filter when nothing is chosen', () => {
    expect(buildDocumentFilter({ docKinds: [] })).toBeUndefined();
    expect(buildChatRequest({ settings: { ...settings }, message: 'hi' }).document_filter).toBeUndefined();
  });

  it('sends JSON without an image and multipart with one', () => {
    const plain = chatRequestBody({ message: 'q' }, true);
    expect(typeof plain).toBe('string');
    expect(JSON.parse(plain as string)).toMatchObject({ message: 'q', stream: true });

    const file = new File([new Uint8Array([1, 2, 3])], 'photo.png', { type: 'image/png' });
    const form = chatRequestBody({ message: 'q', query_image: file }, false);
    expect(form).toBeInstanceOf(FormData);
    const data = form as FormData;
    expect((data.get('query_image') as File).name).toBe('photo.png');
    const json = JSON.parse(data.get('request') as string);
    expect(json).toMatchObject({ message: 'q', stream: false });
    expect(json).not.toHaveProperty('query_image');
  });
});
