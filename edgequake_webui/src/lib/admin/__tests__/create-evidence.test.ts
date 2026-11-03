import { describe, expect, it } from 'vitest';
import {
  evidenceChoices,
  imageEmbeddingPin,
  imagesUnavailable,
  resolveCreateModalities,
  GRAPH_EVIDENCE_REASON,
} from '../create-evidence';

describe('SPEC-183 create evidence', () => {
  it('never sends images for a graph workspace', () => {
    // The dialog must not offer a pair the server refuses (`modalities_require_normal`).
    expect(resolveCreateModalities('graph', 'text,image')).toBe('text');
    expect(resolveCreateModalities('graph', 'text')).toBe('text');
  });

  it('keeps the image choice on a normal workspace', () => {
    expect(resolveCreateModalities('normal', 'text,image')).toBe('text,image');
    expect(resolveCreateModalities('normal', 'text')).toBe('text');
  });

  it('disables images on graph and says why, instead of hiding them', () => {
    const [text, images] = evidenceChoices('graph');
    expect(text!.value).toBe('text');
    expect(text!.disabled).toBe(false);
    expect(images!.disabled).toBe(true);
    expect(images!.reason).toBe(GRAPH_EVIDENCE_REASON);
  });

  it('offers both choices on normal', () => {
    const choices = evidenceChoices('normal');
    expect(choices.map((c) => c.value)).toEqual(['text', 'text,image']);
    expect(choices.every((c) => !c.disabled && c.reason === null)).toBe(true);
  });

  it('reports images unavailable when no offered model can read them', () => {
    expect(
      imagesUnavailable({ retrieval: 'normal', modalities: 'text,image', imageModelOffered: false }),
    ).toBe(true);
    expect(
      imagesUnavailable({ retrieval: 'normal', modalities: 'text,image', imageModelOffered: true }),
    ).toBe(false);
    // Text-only never needs an image model.
    expect(
      imagesUnavailable({ retrieval: 'normal', modalities: 'text', imageModelOffered: false }),
    ).toBe(false);
    // Graph is clamped to text before this is asked.
    expect(
      imagesUnavailable({ retrieval: 'graph', modalities: 'text,image', imageModelOffered: false }),
    ).toBe(false);
  });
});

const textOnlyModel = { provider: 'openai', model: 'text-embedding-3-small', supports_vision: false };
const imageModel = { provider: 'vertexai', model: 'gemini-embedding-2', supports_vision: true };

describe('SPEC-183 image embedding pin', () => {
  it('pins the first image-capable model when images are chosen and the pair cannot read them', () => {
    // Without this the operator is left on a disabled Create beside a text-only embedder — the
    // same dead end, one screen later.
    expect(
      imageEmbeddingPin({
        retrieval: 'normal',
        modalities: 'text,image',
        current: textOnlyModel,
        offered: [textOnlyModel, imageModel],
        pairReadsImages: false,
      }),
    ).toEqual({ provider: 'vertexai', model: 'gemini-embedding-2' });
  });

  it('leaves a pair that already reads images alone', () => {
    expect(
      imageEmbeddingPin({
        retrieval: 'normal',
        modalities: 'text,image',
        current: imageModel,
        offered: [textOnlyModel, imageModel],
        pairReadsImages: true,
      }),
    ).toBeNull();
  });

  it('does not guess when the offer has no image-capable model', () => {
    expect(
      imageEmbeddingPin({
        retrieval: 'normal',
        modalities: 'text,image',
        current: textOnlyModel,
        offered: [textOnlyModel],
        pairReadsImages: false,
      }),
    ).toBeNull();
  });

  it('does nothing for text evidence or a graph workspace', () => {
    for (const retrieval of ['normal', 'graph'] as const) {
      expect(
        imageEmbeddingPin({
          retrieval,
          modalities: 'text',
          current: textOnlyModel,
          offered: [imageModel],
          pairReadsImages: false,
        }),
      ).toBeNull();
    }
    expect(
      imageEmbeddingPin({
        retrieval: 'graph',
        modalities: 'text,image',
        current: textOnlyModel,
        offered: [imageModel],
        pairReadsImages: false,
      }),
    ).toBeNull();
  });

  it('does not re-pin the pair that is already pinned', () => {
    expect(
      imageEmbeddingPin({
        retrieval: 'normal',
        modalities: 'text,image',
        current: imageModel,
        offered: [imageModel],
        pairReadsImages: false,
      }),
    ).toBeNull();
  });
});
