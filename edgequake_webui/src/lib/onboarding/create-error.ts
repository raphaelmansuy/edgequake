/**
 * SPEC-179 — turn a refused create/apply into words and a place to fix it.
 * Codes are the stable ones the model gate and workspace create return.
 */

export interface CreateFailure {
  title: string;
  description: string;
  /** Wizard step that fixes it, when there is one. */
  step?: 'models';
}

const MODEL_STEP = 'models' as const;

const BY_CODE: Record<string, Omit<CreateFailure, 'description'> & { description?: string }> = {
  model_not_eligible: {
    title: 'A chosen model is no longer offered',
    description: 'Pick another model. The organization or deployment changed what it offers.',
    step: MODEL_STEP,
  },
  capability_mismatch: {
    title: 'A chosen model cannot do that job',
    description: 'The vision model must read images. The embedding model must produce vectors.',
    step: MODEL_STEP,
  },
  model_pair_incomplete: {
    title: 'A model needs its provider',
    description: 'Choose a provider and a model together.',
    step: MODEL_STEP,
  },
  embedding_dimension_required: {
    title: 'The embedding model has no registered size',
    description: 'Ask a deployment admin to register its dimension, or pick another embedding model.',
    step: MODEL_STEP,
  },
  credential_missing: {
    title: 'The provider has no working key',
    description: 'Ask a deployment admin to fix the connection, or pick another provider.',
    step: MODEL_STEP,
  },
  embedding_model_not_multimodal: {
    title: 'The embedding model cannot read images',
    description: 'Pick an embedding model that accepts images, or choose Text only.',
    step: MODEL_STEP,
  },
  /**
   * SPEC-180 11 "Create probe failing": the deployment cannot call the image embedding
   * model (usually a project the identity has no access to). The fix lives on the model
   * step, so the wizard goes back there instead of leaving the user on Review with a
   * vendor stack trace.
   */
  embedding_auth: {
    title: 'This deployment cannot embed images yet',
    description:
      'The image embedding call was refused. Choose a different image embedding model, or ask a deployment admin to fix the provider credentials.',
    step: MODEL_STEP,
  },
  embedding_request_failed: {
    title: 'The embedding model did not answer',
    description: 'The provider could not be reached. Try again, or pick another embedding model.',
    step: MODEL_STEP,
  },
  image_too_large: {
    title: 'The probe image was rejected',
    description: 'The embedding endpoint refused the image. Check its size limits.',
    step: MODEL_STEP,
  },
  offer_unavailable: {
    title: 'Provider choices are unavailable right now',
    description: 'Nothing was created. Try again in a moment.',
  },
};

export function describeCreateFailure(error: unknown): CreateFailure {
  const code = (error as { code?: string } | null)?.code;
  // The API client throws an `Error`; a test or a plain rejection may carry `message` alone.
  const asObject = error as { message?: unknown } | null;
  const message =
    error instanceof Error
      ? error.message
      : typeof asObject?.message === 'string'
        ? asObject.message
        : '';
  const known = code ? BY_CODE[code] : undefined;
  if (known) {
    return {
      title: known.title,
      description: message || known.description || '',
      step: known.step,
    };
  }
  return {
    title: 'Failed to create workspace',
    description: message || 'Unknown error',
  };
}
