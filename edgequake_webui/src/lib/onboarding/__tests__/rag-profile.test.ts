import { describe, expect, it } from 'vitest';
import { capabilitiesForDraft, capabilitiesOf, parseRagProfile } from '@/constants/rag-profile';
import { buildWorkspaceIngestPayload } from '../model-payload';
import { profileCreateFields, withoutImmutableProfileFields } from '../rag-profile-payload';
import {
  draftEmbedsImages,
  imageEmbeddingOk,
  patchForModalities,
  patchForProfile,
} from '../retrieval-draft';
import { describeCreateFailure } from '../create-error';
import { describeBlocker, type BlockerTranslate } from '../wizard-blocker-message';
import { EMPTY_WIZARD_DRAFT, wizardBlocker, type WizardDraft } from '../wizard-state';

const normal: WizardDraft = { ...EMPTY_WIZARD_DRAFT, ragProfile: 'normal' };
const withImages: WizardDraft = { ...normal, embeddingModalities: 'text,image' };
const imageModel = {
  provider: 'vertexai',
  model: 'gemini-embedding-2',
  dimension: 768,
  embedsImages: true,
};

describe('rag profile capabilities', () => {
  it('defaults an unknown wire value to graph', () => {
    expect(parseRagProfile('nonsense')).toBe('graph');
    expect(parseRagProfile(undefined)).toBe('graph');
  });

  it('a normal workspace has no graph UI and a graph workspace has all of it', () => {
    const n = capabilitiesForDraft('normal', 'text');
    expect(n.shows_graph_ui).toBe(false);
    expect(n.extracts_graph).toBe(false);
    expect(n.embeds_images).toBe(false);
    expect(capabilitiesForDraft('normal', 'text,image').embeds_images).toBe(true);
    expect(capabilitiesForDraft('graph', 'text').shows_graph_ui).toBe(true);
  });

  it('prefers the capabilities the server sent', () => {
    const server = { ...capabilitiesForDraft('normal', 'text'), embeds_images: true };
    expect(capabilitiesOf({ rag_profile_capabilities: server })).toEqual(server);
  });

  it('reads a workspace without a profile as graph', () => {
    expect(capabilitiesOf({}).shows_graph_ui).toBe(true);
  });
});

describe('retrieval draft patches', () => {
  it('choosing graph resets evidence to text and falls back to defaults when nothing is picked', () => {
    expect(patchForProfile(withImages, 'graph')).toEqual({
      ragProfile: 'graph',
      embeddingModalities: 'text',
      // An image draft is forced into custom mode; with no pick left, inherit again.
      useServerDefaults: true,
    });
  });

  it('choosing the same profile changes nothing', () => {
    expect(patchForProfile(normal, 'normal')).toEqual({});
  });

  it('a graph draft cannot pick images', () => {
    expect(patchForModalities(EMPTY_WIZARD_DRAFT, 'text,image')).toEqual({});
  });

  it('picking images drops a text-only embedding pick', () => {
    const draft = {
      ...normal,
      embeddingPick: { provider: 'openai', model: 'text-embedding-3-small', dimension: 1536 },
    } as WizardDraft;
    expect(patchForModalities(draft, 'text,image')).toEqual({
      embeddingModalities: 'text,image',
      embeddingPick: undefined,
    });
  });

  it('going back to Text from images returns to inherited defaults when nothing else is picked', () => {
    // The image flow forces custom mode; without a pick the models step had no valid state.
    const draft = { ...withImages, useServerDefaults: false } as WizardDraft;
    expect(patchForModalities(draft, 'text')).toEqual({
      embeddingModalities: 'text',
      useServerDefaults: true,
    });
  });

  it('going back to Text keeps a deliberate LLM override in custom mode', () => {
    const draft = {
      ...withImages,
      useServerDefaults: false,
      llmPick: { provider: 'openai', model: 'gpt-5-nano' },
    } as WizardDraft;
    expect(patchForModalities(draft, 'text')).toEqual({ embeddingModalities: 'text' });
  });

  it('keeps an image-capable pick', () => {
    const draft = { ...normal, embeddingPick: imageModel } as WizardDraft;
    expect(patchForModalities(draft, 'text,image')).toEqual({ embeddingModalities: 'text,image' });
  });

  it('requires an image-capable embedding only for text and images', () => {
    expect(draftEmbedsImages(normal)).toBe(false);
    expect(draftEmbedsImages(withImages)).toBe(true);
    expect(imageEmbeddingOk(normal)).toBe(true);
    expect(imageEmbeddingOk(withImages)).toBe(false);
    expect(imageEmbeddingOk({ ...withImages, embeddingPick: imageModel } as WizardDraft)).toBe(true);
  });
});

describe('create payload', () => {
  it('a graph draft sends its profile and the graph fields', () => {
    const payload = buildWorkspaceIngestPayload(EMPTY_WIZARD_DRAFT, 'create') as Record<string, unknown>;
    expect(payload.rag_profile).toBe('graph');
    expect(payload.embedding_modalities).toBe('text');
  });

  it('a normal draft sends no graph-only field', () => {
    const payload = buildWorkspaceIngestPayload(
      { ...normal, entityTypes: ['PERSON'], extractionLanguage: 'French' },
      'create',
    ) as Record<string, unknown>;
    expect(payload.rag_profile).toBe('normal');
    for (const key of ['entity_types', 'extraction_language', 'extraction_mode', 'extract_budget_mode']) {
      expect(payload).not.toHaveProperty(key);
    }
  });

  it('never sends text,image for a graph draft', () => {
    expect(
      profileCreateFields({ ...EMPTY_WIZARD_DRAFT, embeddingModalities: 'text,image' })
        .embedding_modalities,
    ).toBe('text');
  });

  it('an update never carries the fields fixed at create', () => {
    const body = withoutImmutableProfileFields({
      name: 'x',
      rag_profile: 'normal',
      embedding_modalities: 'text',
      fts_regconfig: 'english',
    });
    expect(body).toEqual({ name: 'x' });
  });
});

describe('gate and errors', () => {
  it('blocks the models step until an image-capable embedding is picked', () => {
    const base = { hasConfiguredDefaults: true, advancedModelsValid: true };
    expect(wizardBlocker('models', withImages, { ...base, imageEmbeddingOk: false })).toBe(
      'embedding_not_multimodal',
    );
    expect(wizardBlocker('models', withImages, { ...base, imageEmbeddingOk: true })).toBeNull();
  });

  it('leaves the multimodal reason to the inline notice so it is not said twice', () => {
    const t = ((_k: string, d: string) => d) as unknown as BlockerTranslate;
    expect(describeBlocker('embedding_not_multimodal', t, {})).toBeNull();
    // The create refusal still names it in words, because there is no picker on screen.
    const failure = describeCreateFailure({ code: 'embedding_model_not_multimodal' });
    expect(failure.description).toMatch(/image/i);
  });

  it('sends a refused embedding model back to the models step', () => {
    const failure = describeCreateFailure({ code: 'embedding_model_not_multimodal' });
    expect(failure.step).toBe('models');
  });

  it('blocks on a deployment that cannot embed images, and says what to fix', () => {
    const base = { hasConfiguredDefaults: true, advancedModelsValid: true, imageEmbeddingOk: true };
    expect(wizardBlocker('models', withImages, { ...base, imageProbeFailed: true })).toBe(
      'embedding_probe_failed',
    );
    const t = ((_k: string, d: string) => d) as unknown as BlockerTranslate;
    const message = describeBlocker('embedding_probe_failed', t, {
      imageProbeMessage: 'Vertex refused the image embedding call for project p: 403',
    });
    expect(message).toContain('403');
  });

  it('waits for the probe instead of racing it to review', () => {
    const base = { hasConfiguredDefaults: true, advancedModelsValid: true };
    expect(wizardBlocker('models', withImages, { ...base, imageProbeChecking: true })).toBe(
      'embedding_probe_checking',
    );
  });

  it('a refused image probe returns the user to the models step', () => {
    const failure = describeCreateFailure({
      code: 'embedding_auth',
      message: 'Vertex refused the image embedding call for project p',
    });
    expect(failure.step).toBe('models');
    expect(failure.description).toContain('project p');
  });
});
