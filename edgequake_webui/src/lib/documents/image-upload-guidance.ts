/**
 * SPEC-183 — what to tell someone who drops an image into a text-only workspace.
 *
 * The server's refusal is correct (`embedding_model_not_multimodal`, EC-180-18: an image is
 * never captioned in its place). What was missing is the other half of a good refusal: where
 * the fix lives. `embedding_modalities` is immutable, so the answer is a *new* workspace with
 * Evidence = Text and images — and the dropzone is exactly where the person finds out.
 *
 * Pure, so the sentence is tested rather than screenshotted.
 */

export interface ImageUploadGuidance {
  /** True when this workspace cannot store images, so the dropzone should say so up front. */
  warns: boolean;
  title: string;
  body: string;
}

export const IMAGE_UPLOAD_NEEDS_IMAGES_TITLE =
  'This workspace stores text only';
export const IMAGE_UPLOAD_NEEDS_IMAGES_BODY =
  'A picture needs a workspace whose evidence is “Text and images”, and that is fixed when the workspace is created. Make a new workspace with it, then upload the pictures there.';

/** The capability the dropzone reads. `embeds_images` absent means "unknown", not "no". */
export type ImageCapability =
  | { embeds_images: boolean }
  | null
  | undefined;

/**
 * The guidance for a workspace, from the capability the server sent.
 *
 * `embeds_images` is the whole answer: graph workspaces are always false, and a normal
 * workspace is false until its immutable modalities include images. No component compares
 * the profile name (LAW-180-2).
 *
 * An absent capability is `null` and warns about nothing: a workspace the server has not
 * described yet, or a surface whose fixture predates this field, must not have its uploads
 * refused on the strength of a guess. Only an explicit `false` blocks.
 */
export function imageUploadGuidance(caps: ImageCapability): ImageUploadGuidance {
  if (caps?.embeds_images !== false) {
    return { warns: false, title: '', body: '' };
  }
  return {
    warns: true,
    title: IMAGE_UPLOAD_NEEDS_IMAGES_TITLE,
    body: IMAGE_UPLOAD_NEEDS_IMAGES_BODY,
  };
}

/** True when a dropped file is a picture this workspace is known to be unable to store. */
export function isBlockedImageUpload(
  fileName: string,
  mimeType: string | undefined,
  caps: ImageCapability,
): boolean {
  // Fail open: the server is the authority on whether a picture can be stored, and it answers
  // with a coded 422. Guessing "no" here would refuse uploads the deployment accepts.
  if (caps?.embeds_images !== false) return false;
  const mime = (mimeType ?? '').toLowerCase();
  if (mime.startsWith('image/')) return true;
  return /\.(png|jpe?g|gif|webp)$/i.test(fileName);
}
